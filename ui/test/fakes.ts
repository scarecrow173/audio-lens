/**
 * headless 検証用の `AlServicePort` 実装(#131)。
 *
 * `AlGuiService` の op 契約を忠実にモデル化する:
 * - `call` は op 名 + params を記録し、実 op 相当の結果 JSON を返す。
 * - `job` は stage 単位で進行制御できる handle を返す(完了・失敗・
 *   cancel をテストが deterministically 駆動する)。
 * - dataset は root ごとの manifest モデル — `advanceDataset` が
 *   artifact_count を進め、overview fingerprint が変わる(stale 検出用)。
 * - 失敗分類は実装のコード体系どおり(`cancelled`/`invalid`/`not_found`/
 *   `stage`/`io`)。
 *
 * これは検証用モデルであり、AlGuiService の実装ロジックの複製ではない
 * — op の形と結果の構造だけを同じにする。
 */

import { AL_OPS, STAGE_NAMES } from "../src/ops.ts";
import type { AlJobHandle, AlServiceError, AlServicePort } from "../src/port.ts";

/** `dispatch`/`binding` の queue flush を同期化するテスト helper。 */
export const flushSync = (fn: () => void): void => fn();

/** binding の ack timer を手動駆動する。 */
export class ManualTimers {
  private seq = 0;
  readonly timers = new Map<number, { fn: () => void; ms: number }>();
  readonly schedule = (fn: () => void, ms: number): (() => void) => {
    const id = ++this.seq;
    this.timers.set(id, { fn, ms });
    return () => this.timers.delete(id);
  };
  runNext(): void {
    const first = [...this.timers.entries()].sort((a, b) => a[0] - b[0])[0];
    if (first === undefined) throw new Error("no pending timer");
    this.timers.delete(first[0]);
    first[1].fn();
  }
  /** `elapsedMs` 以内に発火する timer をすべて実行する(遅延 receipt の
   *  検証で「到着しない時間」と「到着する時間」を分ける)。 */
  runDue(elapsedMs: number): void {
    const due = [...this.timers.entries()].filter(([, t]) => t.ms <= elapsedMs);
    for (const [id, t] of due) {
      this.timers.delete(id);
      t.fn();
    }
  }
  get pending(): number {
    return this.timers.size;
  }
}

export class FakeJobHandle implements AlJobHandle {
  readonly id: string;
  private readonly stages: string[];
  private completed = 0;
  private readonly progressCbs = new Set<(d: number, t: number) => void>();
  private resolve!: (v: unknown) => void;
  private reject!: (e: unknown) => void;
  readonly done: Promise<unknown>;
  private settled = false;
  /** 失敗させる stage を指定した場合、その index で stage error になる。 */
  failAt?: number;

  private readonly jobId: string;

  constructor(id: string, stages: readonly string[], jobId: string) {
    this.id = id;
    this.stages = [...stages];
    this.jobId = jobId;
    this.done = new Promise((res, rej) => {
      this.resolve = res;
      this.reject = rej;
    });
  }

  get completedStages(): readonly { stage: string; status: string; error?: string | null }[] {
    return this.stages.map((stage, i) => ({
      stage,
      status: i < this.completed ? "completed" : i === this.completed ? "pending" : "pending",
      error: null,
    }));
  }

  onProgress(cb: (done: number, total: number) => void): void {
    this.progressCbs.add(cb);
  }

  /** stage を 1 つ完了させる(進捗通知あり)。 */
  advance(): void {
    if (this.settled || this.completed >= this.stages.length) return;
    this.completed += 1;
    for (const cb of this.progressCbs) cb(this.completed, this.stages.length);
    if (this.failAt !== undefined && this.completed === this.failAt) {
      this.settled = true;
      // 実 ErrorBody は `{code, message}` のみ(data フィールドを持た
      // ない — stage 名は message に含まれる)。
      this.reject({
        code: "stage",
        message: `stage ${this.stages[this.completed - 1]} failed`,
      } satisfies AlServiceError);
      return;
    }
    if (this.completed === this.stages.length) {
      this.settled = true;
      this.resolve({
        job_id: this.jobId,
        stages: this.completedStages.map((s) => ({ ...s, status: "completed" })),
      });
    }
  }

  /** 全 stage を一気に完了させる。 */
  finish(): void {
    // 0 stage の job(`stages: []` 指定)は即 settled — advance() は
    // 1 件も進められず while が空転するため、ここで解決する。
    if (this.stages.length === 0 && !this.settled) {
      this.settled = true;
      this.resolve({ job_id: this.jobId, stages: [] });
      return;
    }
    while (!this.settled) this.advance();
  }

  cancel(): void {
    if (this.settled) return;
    this.settled = true;
    // 実 op は cancel 時 job_id を返さない(ErrorBody `{code,message}`
    // のみ) — job の再発見は `al.job.list` 経路。
    this.reject({
      code: "cancelled",
      message: "job cancelled",
    } satisfies AlServiceError);
  }
}

interface FakeDataset {
  root: string;
  artifactCount: number;
  inputs: { input_id: string; role: string }[];
  sampleRate: number;
  channels: number;
  durationS: number;
  /** `al.ds.read` table 応答の行(f64 を含む dataset 値)。 */
  featureRows: Record<string, unknown>[];
}

interface FakeJob {
  jobId: string;
  inputs: { role: string; path: string }[];
  stages: { stage: string; status: string; error?: string | null }[];
  datasetReady: boolean;
}

export class FakeAlService implements AlServicePort {
  private nextSessionId = 0;
  private nextJobSeq = 0;
  private nextHandleSeq = 0;
  readonly sessions = new Map<number, string>();
  readonly datasets = new Map<string, FakeDataset>();
  readonly jobs = new Map<string, FakeJob>();
  readonly running = new Map<string, FakeJobHandle>();

  /** 呼出の観測ログ `{op, params}` — op が正しく届いたか検証する。 */
  readonly calls: { op: string; params: Record<string, unknown> }[] = [];
  /** query の明示 cancel が来た queryId。 */
  readonly cancelledQueries: string[] = [];
  /** dataset root の登録。 */
  addDataset(ds: Partial<FakeDataset> & { root: string }): FakeDataset {
    const full: FakeDataset = {
      root: ds.root,
      artifactCount: ds.artifactCount ?? 4,
      inputs: ds.inputs ?? [{ input_id: "in-01", role: "capture_bug" }],
      sampleRate: ds.sampleRate ?? 48000,
      channels: ds.channels ?? 2,
      durationS: ds.durationS ?? 60,
      featureRows: ds.featureRows ?? [],
    };
    this.datasets.set(ds.root, full);
    return full;
  }

  /** dataset の進展(再解析相当)— overview fingerprint を変える。 */
  advanceDataset(root: string): void {
    const ds = this.datasets.get(root);
    if (ds === undefined) throw new Error(`no dataset ${root}`);
    ds.artifactCount += 1;
  }

  /** 登録済み job(job.list / job.inspect 対象)。 */
  addJob(job: FakeJob): FakeJob {
    this.jobs.set(job.jobId, job);
    return job;
  }

  async call(op: string, params: Record<string, unknown> = {}): Promise<unknown> {
    this.calls.push({ op, params });
    switch (op) {
      case AL_OPS.appConfigure:
        return { base_dir: params.base_dir };
      case AL_OPS.appInfo:
        return { name: "al-gui", version: "0.1.0-fake", base_dir: params.base_dir ?? null };
      case AL_OPS.jobList:
        return {
          jobs: [...this.jobs.values()].map((j) => ({
            job_id: j.jobId,
            inputs: j.inputs,
            stages: j.stages,
            dataset_ready: j.datasetReady,
          })),
        };
      case AL_OPS.jobInspect: {
        const job = this.jobs.get(String(params.job_id));
        if (job === undefined) {
          throw {
            code: "not_found",
            message: `job ${params.job_id} not found`,
          } satisfies AlServiceError;
        }
        return {
          job_id: job.jobId,
          manifest: { job_id: job.jobId, stages: job.stages, inputs: job.inputs },
          dataset_ready: job.datasetReady,
          dataset_artifacts: job.datasetReady ? 4 : null,
        };
      }
      case AL_OPS.dsOpen: {
        const root =
          params.root ??
          (params.job_id !== undefined ? `job-${params.job_id}-analysis` : undefined);
        if (root === undefined || !this.datasets.has(String(root))) {
          throw {
            code: "not_found",
            message: `dataset root ${root ?? "?"} not found`,
          } satisfies AlServiceError;
        }
        this.nextSessionId += 1;
        this.sessions.set(this.nextSessionId, String(root));
        return { session: this.nextSessionId, root: String(root) };
      }
      case AL_OPS.dsClose: {
        this.sessions.delete(Number(params.session));
        return { closed: Number(params.session) };
      }
      case AL_OPS.dsOverview: {
        const ds = this.datasetFor(params.session);
        // 実応答形 `{overview, timeline}`(service.rs)。
        return {
          overview: {
            root: ds.root,
            schema_version: 3,
            audio_format: "wav",
            inputs: ds.inputs,
            roles: { inputs: { status: "produced", artifact_count: ds.artifactCount } },
            artifact_count: ds.artifactCount,
          },
          timeline: { inputs: ds.inputs.length, sample_rate: ds.sampleRate },
        };
      }
      case AL_OPS.dsEvents: {
        this.datasetFor(params.session);
        return { events: [], candidates: [] };
      }
      case AL_OPS.dsLocate:
        return { locate: [], details: [] };
      case AL_OPS.dsFeatures: {
        this.datasetFor(params.session);
        const roles = params.roles as string[] | undefined;
        return {
          features: [
            { feature_id: "f-1", name: "rms", role: roles?.[0] ?? "features/cheap", value: 0.5 },
          ],
        };
      }
      case AL_OPS.dsArtifacts:
        return { artifacts: [] };
      case AL_OPS.dsArtifact:
        return { artifact: {} };
      case AL_OPS.dsRead: {
        const ds = this.datasetFor(params.session);
        if (params.kind !== "table" || params.role === undefined || params.name === undefined) {
          throw {
            code: "invalid",
            message: "ds.read requires kind/role/name",
          } satisfies AlServiceError;
        }
        return {
          role: params.role,
          name: params.name,
          row_start: params.row_start ?? 0,
          row_end: params.row_end ?? ds.featureRows.length,
          rows: ds.featureRows,
        };
      }
      case AL_OPS.viewWaveform: {
        const ds = this.datasetFor(params.session);
        const bins = Number(params.bins ?? 1024);
        const bytes = bins * 8 * Math.max(1, Number(params.channel !== undefined ? 1 : 1));
        return {
          source: `al/waveform/${params.session}`,
          bytes,
          meta: {
            input_id: params.input_id,
            channel: params.channel ?? 0,
            bins,
            sample_rate: ds.sampleRate,
          },
        };
      }
      case AL_OPS.viewSpectrogram: {
        const ds = this.datasetFor(params.session);
        return {
          source: `al/spectrogram/${params.session}`,
          bytes: 4096,
          meta: {
            input_id: params.input_id,
            channel: params.channel ?? 0,
            sample_rate: ds.sampleRate,
          },
        };
      }
      default:
        throw { code: "invalid", message: `unknown op ${op}` } satisfies AlServiceError;
    }
  }

  private datasetFor(session: unknown): FakeDataset {
    const root = this.sessions.get(Number(session));
    if (root === undefined) {
      throw { code: "invalid", message: `no dataset session ${session}` } satisfies AlServiceError;
    }
    const ds = this.datasets.get(root);
    if (ds === undefined) {
      throw { code: "not_found", message: `dataset ${root} gone` } satisfies AlServiceError;
    }
    return ds;
  }

  job(op: "al.job.run" | "al.job.resume", params: Record<string, unknown>): FakeJobHandle {
    this.calls.push({ op, params });
    // 実 service は未知 job_id の resume を not_found で失敗させる。
    if (op === "al.job.resume" && !this.jobs.has(String(params.job_id))) {
      throw {
        code: "not_found",
        message: `job ${params.job_id} not found`,
      } satisfies AlServiceError;
    }
    this.nextJobSeq += 1;
    this.nextHandleSeq += 1;
    const jobId =
      op === "al.job.resume"
        ? String(params.job_id)
        : `job-${String(this.nextJobSeq).padStart(4, "0")}`;
    const stages =
      op === "al.job.run"
        ? ((params.stages as readonly string[] | null | undefined) ?? [...STAGE_NAMES])
        : (this.jobs.get(jobId)?.stages.map((s) => s.stage) ?? [...STAGE_NAMES]);
    const handle = new FakeJobHandle(`handle-${this.nextHandleSeq}`, stages, jobId);
    this.running.set(handle.id, handle);
    if (op === "al.job.run") {
      this.jobs.set(jobId, {
        jobId,
        inputs: (params.inputs as { role: string; path: string }[] | undefined) ?? [],
        stages: stages.map((s) => ({ stage: s, status: "pending", error: null })),
        datasetReady: false,
      });
    }
    return handle;
  }

  /** 実行中 job を取得して stage を駆動する。 */
  runningJob(id?: string): FakeJobHandle {
    const h = id === undefined ? [...this.running.values()].at(-1) : this.running.get(id);
    if (h === undefined) throw new Error("no running job");
    return h;
  }

  /** cancelQuery の記録。 */
  cancelQuery(queryId: string): void {
    this.cancelledQueries.push(queryId);
  }
}

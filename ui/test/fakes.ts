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

export interface FakeDataset {
  root: string;
  artifactCount: number;
  inputs: { input_id: string; role: string }[];
  sampleRate: number;
  channels: number;
  durationS: number;
  /** `al.ds.read` table 応答の行(f64 を含む dataset 値)。 */
  featureRows: Record<string, unknown>[];
  /** `al.ds.read` kind=`json` 応答 — artifact path
   *  (`locate/<src>/<cap>/time_map.json` 等)→ JSON 値。 */
  jsonArtifacts: Record<string, unknown>;
  /** `al.ds.events` rows(`events/candidates` 相当)。 */
  eventRows: Record<string, unknown>[];
  /** `al.ds.locate` `alignments` 行(`locate/summary` 相当)。 */
  locateRows: Record<string, unknown>[];
  /** `al.ds.locate` `detail_paths`(verbatim)。 */
  locateDetailPaths: { path: string; provenance: unknown }[];
  /** `al.ds.locate_detail` 応答(`"<src>/<cap>"` → detail)。 */
  locateDetails: Record<string, unknown>;
  /** timeline.inputs の override(manifest input だが audio artifact
   *  無し = missing track をモデル化する場合)。 */
  timelineInputs?: Record<string, unknown>[];
  /** `al.ds.features` の verbatim record 群。 */
  featureRecords: Record<string, unknown>[];
  /** `al.ds.artifacts` 応答(verbatim)。 */
  artifactList: Record<string, unknown>[];
  /** `al.ds.artifact` path → detail。 */
  artifactDetails: Record<string, unknown>;
  /** STFT grid(meta 生成用 — `analysis.cheap.stft_window/hop` 相当)。 */
  stftWindow: number;
  stftHop: number;
  stftBins: number;
  /** RVBW bin の値生成 — NaN を返すと count=0 の gap になる。 */
  waveformFn?: (bin: number, channel: number) => number;
  /** RVST-f32 tile の値生成(row-major、row 0 = 最低 Hz)。 */
  spectrogramFn?: (row: number, col: number) => number;
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
  private nextPublishSeq = 0;
  readonly sessions = new Map<number, string>();
  readonly datasets = new Map<string, FakeDataset>();
  readonly jobs = new Map<string, FakeJob>();
  readonly running = new Map<string, FakeJobHandle>();
  /** `rvf://data` 相当の publish payload レジストリ(#132)。 */
  readonly data = new Map<string, Uint8Array>();

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
      jsonArtifacts: ds.jsonArtifacts ?? {},
      eventRows: ds.eventRows ?? [],
      locateRows: ds.locateRows ?? [],
      locateDetailPaths: ds.locateDetailPaths ?? [],
      locateDetails: ds.locateDetails ?? {},
      ...(ds.timelineInputs !== undefined ? { timelineInputs: ds.timelineInputs } : {}),
      featureRecords: ds.featureRecords ?? [],
      artifactList: ds.artifactList ?? [],
      artifactDetails: ds.artifactDetails ?? {},
      stftWindow: ds.stftWindow ?? 2048,
      stftHop: ds.stftHop ?? 1024,
      stftBins: ds.stftBins ?? 1025,
      ...(ds.waveformFn !== undefined ? { waveformFn: ds.waveformFn } : {}),
      ...(ds.spectrogramFn !== undefined ? { spectrogramFn: ds.spectrogramFn } : {}),
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
        // 実応答形 `{overview, timeline}`(service.rs)。timeline.inputs
        // は audio artifact 毎の `{input_id,frames,sample_rate,channels,
        // duration_s}`。
        return {
          overview: {
            root: ds.root,
            schema_version: 3,
            audio_format: "wav",
            inputs: ds.inputs,
            roles: { inputs: { status: "produced", artifact_count: ds.artifactCount } },
            artifact_count: ds.artifactCount,
          },
          timeline: {
            inputs:
              ds.timelineInputs ??
              ds.inputs.map((i) => ({
                input_id: i.input_id,
                frames: Math.round(ds.durationS * ds.sampleRate),
                sample_rate: ds.sampleRate,
                channels: ds.channels,
                duration_s: ds.durationS,
              })),
            event_count: ds.eventRows.length,
            locate_count: ds.locateRows.length,
          },
        };
      }
      case AL_OPS.dsEvents: {
        const ds = this.datasetFor(params.session);
        // 実応答形 `{row_count, rows}` — `start_s`/`end_s` フィルタは
        // 行の `start_s` 列に対して適用される(service.rs 準拠)。
        const rowStart = Number(params.row_start ?? 0);
        const rowEnd = Math.min(ds.eventRows.length, Number(params.row_end ?? ds.eventRows.length));
        const s = params.start_s !== undefined ? Number(params.start_s) : undefined;
        const e = params.end_s !== undefined ? Number(params.end_s) : undefined;
        const rows = ds.eventRows.slice(rowStart, rowEnd).filter((r) => {
          const rs = typeof r.start_s === "number" ? (r.start_s as number) : undefined;
          if (s !== undefined && rs !== undefined && rs < s) return false;
          if (e !== undefined && rs !== undefined && rs >= e) return false;
          return true;
        });
        return { row_count: ds.eventRows.length, rows };
      }
      case AL_OPS.dsLocate: {
        const ds = this.datasetFor(params.session);
        // 実応答形 `{alignments, detail_paths}`。
        return {
          alignments: ds.locateRows,
          detail_paths: ds.locateDetailPaths,
        };
      }
      case AL_OPS.dsLocateDetail: {
        const ds = this.datasetFor(params.session);
        const key = `${params.source}/${params.capture}`;
        const detail = ds.locateDetails[key];
        if (detail === undefined) {
          throw {
            code: "not_found",
            message: `no locate detail ${key}`,
          } satisfies AlServiceError;
        }
        return detail;
      }
      case AL_OPS.dsFeatures: {
        const ds = this.datasetFor(params.session);
        // 実応答形 `{records, truncated, limit}` — record は verbatim
        // + service が `_artifact` を付与するのと同じ規約で付ける。
        const roles = params.roles as string[] | undefined;
        const namePrefix = params.name_prefix as string | undefined;
        const fidPrefix = params.feature_id_prefix as string | undefined;
        const limit = Number(params.limit ?? 500);
        const records: Record<string, unknown>[] = [];
        let truncated = false;
        for (const [i, rec] of ds.featureRecords.entries()) {
          const role = String(rec._role ?? "features/cheap");
          if (roles !== undefined && !roles.includes(role)) continue;
          const name = String(rec.name ?? rec.feature_id ?? `f-${i}`);
          if (namePrefix !== undefined && !name.startsWith(namePrefix)) continue;
          const fid = String(rec.feature_id ?? "");
          if (fidPrefix !== undefined && !fid.startsWith(fidPrefix)) continue;
          records.push({
            ...rec,
            _artifact: String(rec._artifact ?? `${role}/${name}.json`),
          });
          // 実 service は push 後に `>= limit` で打ち切る — ちょうど
          // limit 件の境界でも truncated: true になる(件数ではなく
          // 打ち切りで判定する規約)。
          if (records.length >= limit) {
            truncated = true;
            break;
          }
        }
        return { records, truncated, limit };
      }
      case AL_OPS.dsArtifacts: {
        const ds = this.datasetFor(params.session);
        const role = params.role as string | undefined;
        return {
          artifacts: ds.artifactList.filter((a) => role === undefined || String(a.role) === role),
        };
      }
      case AL_OPS.dsArtifact: {
        const ds = this.datasetFor(params.session);
        const detail = ds.artifactDetails[String(params.path)];
        if (detail === undefined) {
          throw {
            code: "not_found",
            message: `no artifact ${params.path}`,
          } satisfies AlServiceError;
        }
        return detail;
      }
      case AL_OPS.dsRead: {
        const ds = this.datasetFor(params.session);
        const role = params.role;
        const name = params.name;
        const kind = params.kind;
        if (role === undefined || name === undefined || (kind !== "table" && kind !== "json")) {
          throw {
            code: "invalid",
            message: "ds.read requires kind/role/name",
          } satisfies AlServiceError;
        }
        if (kind === "json") {
          // 実 `DatasetReader::read_json` は name に `.json` を補完し
          // `<role>/<name>` の Json artifact を引く。未登録は
          // InvalidData io error → wire code `io`。
          const rel = `${String(role)}/${String(name)}`;
          const key = rel.endsWith(".json") ? rel : `${rel}.json`;
          const body = ds.jsonArtifacts[key];
          if (body === undefined) {
            throw {
              code: "io",
              message: `no Json artifact at ${key}`,
            } satisfies AlServiceError;
          }
          return body;
        }
        const rowStart = Math.max(0, Number(params.row_start ?? 0));
        const rowEnd = Math.min(
          ds.featureRows.length,
          Number(params.row_end ?? ds.featureRows.length),
        );
        return {
          role: params.role,
          name: params.name,
          row_start: rowStart,
          row_end: rowEnd,
          rows: ds.featureRows.slice(rowStart, rowEnd),
        };
      }
      case AL_OPS.viewWaveform: {
        const ds = this.datasetFor(params.session);
        // 実 service は `WaveformSeries::view(channel,start,end,bins)` の
        // `LodBytes` を publish する — 同じ covered range 計算で
        // `RVBW` payload を生成する。
        const channel = Number(params.channel ?? 0);
        // 実 service は `channel >= info.channels` を invalid で落とす
        // (dataset.rs) — 別 channel の truth を返させない。
        if (!Number.isInteger(channel) || channel < 0 || channel >= ds.channels) {
          throw {
            code: "invalid",
            message: `channel ${channel} out of range (${ds.channels})`,
          } satisfies AlServiceError;
        }
        const bins = Math.max(1, Number(params.bins ?? 1024));
        const rate = ds.sampleRate;
        const framesTotal = Math.round(ds.durationS * rate);
        const startF = Math.min(
          framesTotal,
          Math.max(0, Math.floor(Number(params.start_s ?? 0) * rate)),
        );
        const endF = Math.min(
          framesTotal,
          Math.max(startF + 1, Math.floor(Number(params.end_s ?? ds.durationS) * rate)),
        );
        const span = (endF - startF) / bins;
        const recs: RvbwBinRec[] = new Array(bins);
        for (let i = 0; i < bins; i++) {
          const v =
            ds.waveformFn !== undefined ? ds.waveformFn(i, channel) : Math.sin(i * 0.25) * 0.5;
          const finite = Number.isFinite(v);
          recs[i] = {
            start: Math.round(startF + i * span),
            end: Math.round(startF + (i + 1) * span),
            min: v,
            max: v,
            rms: v,
            count: finite ? Math.max(1, Math.round(span)) : 0,
          };
        }
        const payload = encodeRvbw(recs);
        // publish 名は実 service と同じ `al/waveform/<seq>` 規約
        // (service.rs) — session は source 名に含まれない。
        const name = `al/waveform/${++this.nextPublishSeq}`;
        this.data.set(name, payload);
        return {
          source: name,
          bytes: payload.byteLength,
          meta: {
            input_id: params.input_id,
            channel,
            sample_rate: rate,
            start: recs[0]?.start ?? 0,
            end: recs[bins - 1]?.end ?? 0,
            bins,
            format: "RVBW",
          },
        };
      }
      case AL_OPS.viewSpectrogram: {
        const ds = this.datasetFor(params.session);
        // 実 service は `Spectrogram::tile({frames,rows,cols})` の RVST-f32
        // を publish する — frame 換算は service.rs と同じ規約
        // (`s * rate / hop`、frame 指定優先)。
        const channel = Number(params.channel ?? 0);
        if (!Number.isInteger(channel) || channel < 0 || channel >= ds.channels) {
          throw {
            code: "invalid",
            message: `channel ${channel} out of range (${ds.channels})`,
          } satisfies AlServiceError;
        }
        const rate = ds.sampleRate;
        const hop = ds.stftHop;
        const fs =
          params.frame_start !== undefined
            ? Number(params.frame_start)
            : Math.floor((Math.max(0, Number(params.start_s ?? 0)) * rate) / hop);
        // `end_s` 未指定は duration まで(秒 — frame 値を秒に誤用
        // しない)。実 `tile()` は source の frame 数へ clamp する。
        const maxFrames = Math.ceil((ds.durationS * rate) / hop);
        const fe = Math.min(
          maxFrames,
          params.frame_end !== undefined
            ? Number(params.frame_end)
            : Math.floor((Number(params.end_s ?? ds.durationS) * rate) / hop),
        );
        const rows = Math.max(1, Number(params.rows ?? 256));
        const cols = Math.max(1, Number(params.cols ?? 1024));
        const values = new Array<number>(rows * cols);
        for (let r = 0; r < rows; r++) {
          for (let c = 0; c < cols; c++) {
            values[r * cols + c] =
              ds.spectrogramFn !== undefined
                ? ds.spectrogramFn(r, c)
                : Math.sin(r * 0.3 + c * 0.05) * 40 - 40;
          }
        }
        const payload = encodeRvstF32(rows, cols, fs, values);
        const name = `al/spectrogram/${++this.nextPublishSeq}`;
        this.data.set(name, payload);
        return {
          source: name,
          bytes: payload.byteLength,
          meta: {
            input_id: params.input_id,
            channel,
            cols,
            rows,
            frame_start: fs,
            frame_end: fe,
            sample_rate: rate,
            hop,
            window: ds.stftWindow,
            bins: ds.stftBins,
            format: "RVST-f32",
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

  /** `rvf://data` 相当の範囲 pull — `data` レジストリから
   *  `offset` 位置の最大 `limit` バイトを返す。unknown source は
   *  `not_found`、EOF は short read。 */
  async pull(source: string, offset: number, limit: number): Promise<Uint8Array> {
    const buf = this.data.get(source);
    if (buf === undefined) {
      throw {
        code: "not_found",
        message: `no published source ${source}`,
      } satisfies AlServiceError;
    }
    const start = Math.min(buf.length, Math.max(0, offset));
    const n = Math.min(limit, buf.length - start);
    return buf.slice(start, start + n);
  }

  /** cancelQuery の記録。 */
  cancelQuery(queryId: string): void {
    this.cancelledQueries.push(queryId);
  }
}

/** `RVBW` の per-bin 入力値(encode helper 用)。 */
export interface RvbwBinRec {
  start: number;
  end: number;
  min: number;
  max: number;
  rms: number;
  count: number;
}

/**
 * `ruviform-viz` `encode_buckets` と同じレイアウトで `RVBW` を emit
 * する(16B header + 48B/bin — `start u64, end u64, min/max/rms f64,
 * count u64`。upstream の `44 B` doc コメントは stale で、実 wire は
 * `count u64` を含む 48B)。`NaN` はそのまま書き込む。
 */
export function encodeRvbw(bins: readonly RvbwBinRec[]): Uint8Array {
  const out = new Uint8Array(16 + bins.length * 48);
  const dv = new DataView(out.buffer);
  out.set([0x52, 0x56, 0x42, 0x57], 0); // "RVBW"
  dv.setUint16(4, 1, true);
  dv.setUint32(8, bins.length, true);
  for (const [i, b] of bins.entries()) {
    const o = 16 + i * 48;
    dv.setBigUint64(o, BigInt(Math.max(0, Math.round(b.start))), true);
    dv.setBigUint64(o + 8, BigInt(Math.max(0, Math.round(b.end))), true);
    dv.setFloat64(o + 16, b.min, true);
    dv.setFloat64(o + 24, b.max, true);
    dv.setFloat64(o + 32, b.rms, true);
    dv.setBigUint64(o + 40, BigInt(Math.max(0, Math.round(b.count))), true);
  }
  return out;
}

/**
 * `encode_tile(.., Quantization::F32)` と同じレイアウトで `RVST`
 * (dtype=1)を emit する(24B header + 4B/elem row-major、row 0 =
 * 最低 Hz)。`NaN` は IEEE NaN として書き込む(f32 精度で保持)。
 */
export function encodeRvstF32(
  rows: number,
  cols: number,
  frameStart: number,
  values: readonly number[],
): Uint8Array {
  if (values.length !== rows * cols) {
    throw new Error(`values ${values.length} !== ${rows}x${cols}`);
  }
  const out = new Uint8Array(24 + rows * cols * 4);
  const dv = new DataView(out.buffer);
  out.set([0x52, 0x56, 0x53, 0x54], 0); // "RVST"
  dv.setUint16(4, 1, true);
  dv.setUint16(6, 1, true); // dtype f32
  dv.setUint32(8, cols, true);
  dv.setUint32(12, rows, true);
  dv.setBigUint64(16, BigInt(Math.max(0, Math.round(frameStart))), true);
  for (let i = 0; i < values.length; i++) {
    dv.setFloat32(24 + i * 4, values[i] ?? Number.NaN, true);
  }
  return out;
}

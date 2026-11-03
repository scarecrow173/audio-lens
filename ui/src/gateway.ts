/**
 * AudioLens `BindingPort` 実装(#131) — `AlGuiGateway`。
 *
 * `ui/binding` の Command/Query binding が要求する 4 つの port
 * (`dispatch`/`subscribe`/`query`/`cancelQuery`)を、`al-gui` の
 * `AlGuiService` ops(#131 経路)と UI 側 view 状態へ橋渡しする。
 *
 * 責務分割(#257 §5 契約どおり):
 * - **binding 側** が担うもの: gesture → intent 変換、preview 結合、
 *   receipt 相関・順序付け・ack timeout、projection の revision guard、
 *   view generation/detach、DOM adapter による element 反映。
 * - **この gateway** が担うもの: AudioLens 側の target 解釈、確定値の
 *   domain 適用、job 起動/cancel の実行、receipt/projection の発行、
 *   dataset source の revision 追跡と stale 応答、missing source /
 *   unsupported op の正直な失敗分類。
 *
 * target 文法(`LogicalTarget`):
 * - `al.view.seek` / `al.view.channel` … scalar view 状態(f64 確定値)
 * - `al.view.range` … `{low, high}` — range-selector の `-lo`/`-hi`
 *   edge が同一 target に commit する
 * - `al.cfg.<path>` … job draft config(`al.cfg.stages.<name>` は stage
 *   有効化、それ以外は config JSON への nested path)
 * - `al.job` … `run`/`resume`/`cancel` action — Job ライフサイクル。
 *   gesture の commit/cancel とは別経路(action command としてのみ解釈)
 * - `al.media.preview` … `play`/`pause`/`resume`/`stop` action —
 *   preview 状態機械。Job とは別の domain。
 *
 * receipt の返し方は 2 経路: `dispatchFor` は binding 側クロージャ
 * (`(r) => binding.ack(r)`)へ直接渡す用途、BindingPort インターフェースの
 * `dispatch` は `onReceipt` コールバック経路(任意設定)へ渡す。
 * commandId は binding ローカル採番で DispatchContext に binding 識別が
 * 無いため、`dispatch` 側では receipt を個別 binding へ demux できない
 * — ack を要する binding は必ず `dispatchFor` 経路で配線すること
 * (`link.createBinding`/`bindControlElement` が作る port 経由の
 * dispatch は receipt を失わせる)。
 */

import type {
  BindingPort,
  CommandIntent,
  CommandReceipt,
  DispatchContext,
  ProjectionListener,
  QueryId,
  QueryReply,
  StateProjection,
  SubscriptionContext,
  ViewportQuery,
} from "../vendor/ruviform/ui/binding/src/index.ts";
import type { AlDsOpenResult, AlRunSpecParams } from "./ops.ts";
import { AL_OPS, STAGE_NAMES } from "./ops.ts";
import { type AlJobHandle, type AlServicePort, isServiceError } from "./port.ts";
import { RevisionedStore } from "./view-state.ts";

const STAGE_SET = new Set<string>(STAGE_NAMES);

export type ReceiptSink = (receipt: CommandReceipt) => void;

/** `al.job`/`al.media.preview` action target の projection 値。 */
export interface JobProjection {
  readonly status: "idle" | "running" | "succeeded" | "failed" | "cancelled";
  readonly job_id?: string;
  readonly stages?: readonly {
    readonly stage: string;
    readonly status: string;
    readonly error?: string | null;
  }[];
  readonly progress?: { readonly done: number; readonly total: number };
  readonly detail?: string;
}

export interface PreviewProjection {
  readonly state: "stopped" | "playing" | "paused";
  readonly position_s: number;
}

/** `al.view.range` target の projection 値。 */
export interface RangeValue {
  readonly low: number;
  readonly high: number;
}

/** dataset session の source 解決結果。 */
export interface SourceHandle {
  readonly kind: string;
  readonly session: number;
  readonly inputId?: string;
  readonly channel?: number;
  /** `spectrogram` source の frequency 軸解像度(rows)。 */
  readonly rows?: number;
  readonly role?: string;
  readonly name?: string;
}

/** run/resume に必要な job draft(確定した cfg 値の集合)。 */
export interface JobDraft {
  inputs: { role: string; path: string; input_id?: string }[];
  stages: Set<string>;
  /** stage 集合が UI/mount によって明示確立されたか。
   *  false のまま空なら `stages` を spec へ載せず service 既定
   *  (全 stage)に委ねる。screen が全 stage を seed した時点、または
   *  stage toggle の commit が 1 度でもあれば true になる — 以後は
   *  空集合も「全 off の明示選択」として `stages: []` を送る。 */
  stagesEdited: boolean;
  config: Map<string, number>;
}

function abortError(): Error {
  const e = new Error("aborted");
  e.name = "AbortError";
  return e;
}

/** `al.ds.overview` 応答から導出する dataset revision token。
 *  manifest 由来の内容 fingerprint — 同じ内容なら同じ token、
 *  何か進展すれば別 token になる(FNV-1a、衝突耐性は検証用途で十分)。 */
export function fingerprint(value: unknown): string {
  const s = JSON.stringify(value) ?? "null";
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return `fp-${(h >>> 0).toString(16)}`;
}

/** `al.cfg.<path>` を config JSON の nested object へ展開する。
 *  `__proto__`/`constructor`/`prototype` を含む path はプロトタイプ
 *  汚染を防ぐため丸ごと捨てる(commit 自体は gateway が reject するが、
 *  直接呼出しへの防衛もここに置く)。scalar/object の衝突は
 *  後勝ち(config overlay の素直な意味づけ)。 */
const FORBIDDEN_CONFIG_KEYS = new Set(["__proto__", "constructor", "prototype"]);
export function nestConfig(entries: Iterable<[string, number]>): Record<string, unknown> {
  const root: Record<string, unknown> = {};
  for (const [path, value] of entries) {
    const keys = path.split(".");
    if (keys.some((k) => FORBIDDEN_CONFIG_KEYS.has(k))) continue;
    let node = root;
    for (const k of keys.slice(0, -1)) {
      const next = Object.hasOwn(node, k) ? node[k] : undefined;
      if (typeof next !== "object" || next === null) node[k] = {};
      node = node[k] as Record<string, unknown>;
    }
    const last = keys.at(-1);
    if (last !== undefined) node[last] = value;
  }
  return root;
}

export class AlGuiGateway implements BindingPort {
  private readonly port: AlServicePort;
  /** BindingPort.dispatch 経路の receipt 送り先(任意)。 */
  onReceipt?: (receipt: CommandReceipt, ctx: DispatchContext) => void;

  /** 全 receipt の発行ログ(監査・検証用)。長期稼働で無制限に
   *  育たないよう直近 512 件に丸める。 */
  readonly receiptLog: CommandReceipt[] = [];
  private static readonly RECEIPT_LOG_MAX = 512;

  private readonly viewStores = new Map<string, RevisionedStore<unknown>>();
  private readonly rangeStore = new RevisionedStore<RangeValue>({ low: 0, high: 0 });
  private readonly jobStore = new RevisionedStore<JobProjection>({ status: "idle" });
  private readonly previewStore = new RevisionedStore<PreviewProjection>({
    state: "stopped",
    position_s: 0,
  });

  readonly draft: JobDraft = {
    inputs: [],
    stages: new Set(),
    stagesEdited: false,
    config: new Map(),
  };

  private baseDir: string | undefined;
  private jobContext: { jobId?: string } = {};
  private jobHandle: { cancel(): void } | undefined;

  /** session → dataset root、root → 現在 revision(fingerprint)。 */
  private readonly sessions = new Map<number, string>();
  private readonly revisions = new Map<string, string>();

  constructor(port: AlServicePort) {
    this.port = port;
  }

  // ── source / revision ────────────────────────────────────────────

  /** `al.ds.open` + `al.ds.overview` で dataset を開き、revision
   *  fingerprint を採取する。fixture は返り値の revision を
   *  `QueryBinding.request` の `sourceRevision` として使う。 */
  async openDataset(
    params: Record<string, unknown>,
  ): Promise<AlDsOpenResult & { revision: string }> {
    const opened = (await this.port.call(AL_OPS.dsOpen, params)) as AlDsOpenResult;
    let overview: unknown;
    try {
      overview = await this.port.call(AL_OPS.dsOverview, {
        session: opened.session,
      });
    } catch (e) {
      // overview 失敗時は開いた session が service 側に残る — 追跡外の
      // 孤児にしないよう best-effort で閉じてから投げ直す。
      await this.port.call(AL_OPS.dsClose, { session: opened.session }).catch(() => undefined);
      throw e;
    }
    const revision = fingerprint(overview);
    this.sessions.set(opened.session, opened.root);
    this.revisions.set(opened.root, revision);
    return { ...opened, revision };
  }

  /** dataset root の現在 revision を再採取する(re-run 後など)。
   *  新しい session で開き直して fingerprint を取り、probe session
   *  は閉じる。view の持つ古い revision との差分が stale 検出になる。 */
  async refreshRevision(root: string): Promise<string> {
    const probe = (await this.port.call(AL_OPS.dsOpen, { root })) as AlDsOpenResult;
    try {
      const overview = await this.port.call(AL_OPS.dsOverview, {
        session: probe.session,
      });
      const revision = fingerprint(overview);
      this.revisions.set(root, revision);
      return revision;
    } finally {
      await this.port.call(AL_OPS.dsClose, { session: probe.session });
    }
  }

  /** `QueryBinding.revisionProbe` 用 — source の現在 revision を返す。 */
  probeRevision(sourceId: string): number | string | undefined {
    const src = this.parseSourceId(sourceId);
    if (src === undefined) return undefined;
    const root = this.sessions.get(src.session);
    return root === undefined ? undefined : this.revisions.get(root);
  }

  // ── BindingPort ─────────────────────────────────────────────────

  /** インターフェース必須の dispatch。receipt は `onReceipt` 経路へ
   *  送る(全 receipt は receiptLog にも残る)。 */
  dispatch(intent: CommandIntent, ctx: DispatchContext): void {
    this.dispatchFor(intent, ctx, (r) => this.onReceipt?.(r, ctx));
  }

  /**
   * binding クロージャ経路の dispatch — receipt を直接 `binding.ack`
   *  へ届ける。screen fixture はこちらを使う(commandId は binding
   *  ローカル採番なので、所有者が分かる経路が正確)。
   */
  dispatchFor(intent: CommandIntent, ctx: DispatchContext, ack: ReceiptSink): void {
    const emit = (
      status: CommandReceipt["status"],
      detail?: string,
      revision?: number | string,
    ) => {
      const receipt: CommandReceipt = {
        commandId: ctx.commandId,
        status,
        ...(revision !== undefined ? { revision } : {}),
        ...(detail !== undefined ? { detail } : {}),
      };
      this.receiptLog.push({
        commandId: ctx.commandId,
        status,
        ...(detail !== undefined ? { detail } : {}),
      });
      if (this.receiptLog.length > AlGuiGateway.RECEIPT_LOG_MAX) this.receiptLog.shift();
      ack(receipt);
    };

    switch (intent.target) {
      case "al.job":
        this.jobIntent(intent, ctx, emit);
        return;
      case "al.media.preview":
        this.previewIntent(intent, ctx, emit);
        return;
      case "al.view.range":
        this.rangeIntent(intent, ctx, emit);
        return;
      default:
        break;
    }

    if (intent.target.startsWith("al.view.") || intent.target.startsWith("al.cfg.")) {
      this.scalarIntent(intent, ctx, emit);
      return;
    }
    // 未知 target: receipt を期待する kind には rejected、それ以外は無視。
    if (ctx.expectReceipt) emit("rejected", `unknown target ${intent.target}`);
  }

  subscribe<P>(
    target: string,
    listener: ProjectionListener<P>,
    _ctx: SubscriptionContext,
  ): (() => void) | undefined {
    if (target === "al.job") {
      return this.jobStore.subscribe(listener as (p: StateProjection<JobProjection>) => void);
    }
    if (target === "al.media.preview") {
      return this.previewStore.subscribe(
        listener as (p: StateProjection<PreviewProjection>) => void,
      );
    }
    if (target === "al.view.range") {
      return this.rangeStore.subscribe(listener as (p: StateProjection<RangeValue>) => void);
    }
    // 未宣言 target はここで lazy 宣言する — 先に subscribe して後から
    // commit される使い方でも projection が永遠に届かない状態にしない。
    const store = this.viewStores.get(target) ?? this.declareTarget(target, 0);
    return store.subscribe(listener as (p: StateProjection<unknown>) => void) as () => void;
  }

  async query<T>(q: ViewportQuery, signal?: AbortSignal): Promise<QueryReply<T>> {
    if (signal?.aborted) throw abortError();
    const src = this.resolveSource(q.sourceId);
    this.validateViewport(src, q);
    const root = this.sessions.get(src.session);
    const currentRev = root === undefined ? undefined : this.revisions.get(root);
    const op = this.sourceOp(src, q);
    // 明示 cancel channel へ abort を橋渡しする(host が持てば
    // サーバ側の in-flight query も止められる)。resolve/abort の
    // どちらが先でも listener は外す。
    const onAbort = (): void => this.cancelQuery(q.queryId);
    signal?.addEventListener("abort", onAbort, { once: true });
    try {
      const result = await this.callWithAbort(op, signal);
      // 現在 revision をそのまま応答へ載せる — view の持つ
      // sourceRevision と違えば binding 側が stale に分類する。
      return {
        generation: q.generation,
        sourceRevision: currentRev ?? q.sourceRevision,
        payload: result as T,
        bytes: replyBytes(result),
      };
    } finally {
      signal?.removeEventListener("abort", onAbort);
    }
  }

  /** viewport 値の domain 検証 — service が `as_u64`/`as_f64` で
   *  黙って default へ落とす値(負 channel・非整数 row・非 finite
   *  秒)を、要求と別物の truth として返さないようここで落とす。 */
  private validateViewport(src: SourceHandle, q: ViewportQuery): void {
    const finite = Number.isFinite(q.start) && Number.isFinite(q.end);
    if (src.kind === "read") {
      const ok = finite && Number.isInteger(q.start) && Number.isInteger(q.end) && q.start >= 0;
      if (!ok) throw new Error(`read viewport must be non-negative integer rows`);
      return;
    }
    if (!finite) throw new Error(`viewport start/end must be finite`);
    if (q.channels !== undefined) {
      // sourceId が channel を内包するため、query 側 channels は
      // 一致する単一指定のみ受理する(複数 ch は未対応 — 黙って
      // 先頭だけ取ると要求と違う truth を返す)。
      if (q.channels.length !== 1 || q.channels[0] !== src.channel) {
        throw new Error(`channels selector unsupported for ${src.kind} source`);
      }
    }
  }

  cancelQuery(queryId: QueryId): void {
    this.port.cancelQuery?.(queryId);
  }

  // ── 画面からの明示操作 ───────────────────────────────────────────

  /** `al.app.configure` 相当 — base_dir を gateway にも覚えさせ、
   *  以降の op 呼出に `base_dir` を明示付与する。 */
  async configure(baseDir: string): Promise<unknown> {
    const out = await this.port.call(AL_OPS.appConfigure, { base_dir: baseDir });
    this.baseDir = baseDir;
    return out;
  }

  /** view/cfg target の scalar store を宣言する(fixture が使う)。 */
  declareTarget(target: string, initial: number): RevisionedStore<unknown> {
    const existing = this.viewStores.get(target);
    if (existing !== undefined) return existing;
    const store = new RevisionedStore<unknown>(initial);
    this.viewStores.set(target, store);
    return store;
  }

  /** 宣言済み scalar store の現在値(read-only 参照用)。 */
  targetValue(target: string): unknown {
    if (target === "al.view.range") return this.rangeStore.value;
    return this.viewStores.get(target)?.value;
  }

  /** 外部 authoritative write — gesture 由来ではない truth 更新
   *  (fixture 初期値、他 window/サービス起因の状態変更)。store commit
   *  が publish も兼ねるため subscriber へ現在値が届き、binding の
   *  revision guard が順序を守る。receipt は発行しない(command に
   *  対する応答ではないため)。形状の合わない値は commit しない
   *  (NaN や `{low:undefined}` を truth にしない)。 */
  applyExternal(target: string, value: unknown): number | undefined {
    if (target === "al.view.range") {
      if (!isRangeValue(value)) return undefined;
      return this.rangeStore.commit({ low: value.low, high: value.high });
    }
    if (target === "al.job" || target === "al.media.preview") {
      return undefined; // job/preview truth は op 結果のみが更新する
    }
    if (typeof value !== "number" || !Number.isFinite(value)) return undefined;
    return this.declareTarget(target, 0).commit(value);
  }

  /** 初回 mount 限定の range 初期値 — remount/別 window 移動で
   *  commit 済みの `{low, high}` を seed で上書きしない。 */
  private rangeSeeded = false;
  seedRange(v: RangeValue): void {
    if (this.rangeSeeded) return;
    this.rangeSeeded = true;
    this.rangeStore.commit({ low: v.low, high: v.high });
  }

  /** resume 対象など job 識別を画面から与える。 */
  setJobContext(ctx: { jobId?: string }): void {
    this.jobContext = ctx;
  }

  get jobProjection(): JobProjection {
    return this.jobStore.value;
  }

  get previewProjection(): PreviewProjection {
    return this.previewStore.value;
  }

  /** run/resume 用の RunSpec params を draft から構築する。
   *  `stages` は「未確立(空 かつ 未編集)」なら省略して service
   *  既定に委ねるが、編集済みの空集合は「全 stage off」の明示選択
   *  として `[]` を送る(省略→全 stage 実行との混同を防ぐ)。 */
  buildRunSpec(): AlRunSpecParams {
    const stages = STAGE_NAMES.filter((s) => this.draft.stages.has(s));
    const params: Record<string, unknown> = {
      inputs: this.draft.inputs,
      config: nestConfig(this.draft.config),
    };
    if (stages.length > 0 || this.draft.stagesEdited) params.stages = stages;
    if (this.baseDir !== undefined) params.base_dir = this.baseDir;
    return params as unknown as AlRunSpecParams;
  }

  // ── dispatch 内処理 ──────────────────────────────────────────────

  /** `al.job`: action のみ受け付ける。gesture 系 intent は
   *  Job ライフサイクルとは別 domain — receipt 期待の commit には
   *  unsupported、lifecycle 系(begin/cancel/preview)は無視する。 */
  private jobIntent(
    intent: CommandIntent,
    ctx: DispatchContext,
    emit: (status: CommandReceipt["status"], detail?: string, revision?: number | string) => void,
  ): void {
    if (intent.kind !== "action") {
      if (ctx.expectReceipt)
        emit("unsupported", `job target accepts actions only, got ${intent.kind}`);
      return;
    }
    switch (intent.action) {
      case "run":
        this.startJob("al.job.run", this.buildRunSpec(), emit);
        return;
      case "resume": {
        const jobId = this.jobContext.jobId;
        if (jobId !== undefined) {
          const params: Record<string, unknown> = { job_id: jobId };
          if (this.baseDir !== undefined) params.base_dir = this.baseDir;
          this.startJob("al.job.resume", params, emit);
          return;
        }
        // job_id 未選択 — cancel 済み run の op は job_id を返さない
        // ため、`al.job.list` で resume 可能 job を再発見する
        // (port.ts の契約)。
        this.resumeViaList(emit);
        return;
      }
      case "cancel": {
        const handle = this.jobHandle;
        if (handle === undefined) {
          emit("rejected", "no running job to cancel");
          return;
        }
        try {
          handle.cancel();
        } catch (e) {
          emit("rejected", `cancel failed: ${String(e)}`);
          return;
        }
        // cancel の receipt は「要求を受理した」まで — domain 結果
        // (cancelled か完了間に合ったか)は done が projection へ届ける。
        emit("accepted", "cancel requested");
        return;
      }
      default:
        emit("unsupported", `unknown job action ${intent.action}`);
        return;
    }
  }

  /** job_id 未指定 resume — `al.job.list` で resume 可能 job を
   *  再発見する。候補が一意でなければ推測で選ばず reject する。 */
  private resumeViaList(
    emit: (status: CommandReceipt["status"], detail?: string, revision?: number | string) => void,
  ): void {
    const params: Record<string, unknown> = {};
    if (this.baseDir !== undefined) params.base_dir = this.baseDir;
    void this.port
      .call(AL_OPS.jobList, params)
      .then((out) => {
        const jobs = (out as { jobs?: unknown }).jobs;
        const list = Array.isArray(jobs)
          ? (jobs as { job_id?: string; dataset_ready?: boolean; stages?: { status?: string }[] }[])
          : [];
        // resume 可能 = 未完了(pending/failed)stage を持ち dataset
        // 未完の job。corrupt manifest(stages 空)は候補から外す。
        const resumable = list.filter(
          (j) =>
            typeof j.job_id === "string" &&
            j.dataset_ready !== true &&
            (j.stages ?? []).some((s) => s.status === "pending" || s.status === "failed"),
        );
        if (resumable.length === 0) {
          emit("rejected", "no resumable job found via al.job.list");
          return;
        }
        if (resumable.length > 1) {
          emit("rejected", `ambiguous resume: ${resumable.length} resumable jobs`);
          return;
        }
        const jobId = resumable[0]?.job_id;
        if (jobId === undefined) {
          emit("rejected", "no resumable job found via al.job.list");
          return;
        }
        const rp: Record<string, unknown> = { job_id: jobId };
        if (this.baseDir !== undefined) rp.base_dir = this.baseDir;
        this.startJob("al.job.resume", rp, emit);
      })
      .catch((e) => {
        emit("rejected", `al.job.list failed: ${isServiceError(e) ? e.message : String(e)}`);
      });
  }

  /** run/resume の起動 — accepted → job 完了で applied/rejected。
   *  同時実行は reject する(1 draft = 1 tracking slot — 2 個目を
   *  走らせると先の job の cancel 経路と projection が壊れる)。 */
  private startJob(
    op: "al.job.run" | "al.job.resume",
    params: AlRunSpecParams | Record<string, unknown>,
    emit: (status: CommandReceipt["status"], detail?: string, revision?: number | string) => void,
  ): void {
    if (this.jobHandle !== undefined) {
      emit("rejected", "a job is already running");
      return;
    }
    let handle: AlJobHandle;
    try {
      handle = this.port.job(op, params as Record<string, unknown>);
    } catch (e) {
      emit("rejected", String(e));
      return;
    }
    this.jobHandle = handle;
    const rev = this.jobStore.commit({ status: "running", progress: { done: 0, total: 1 } });
    emit("accepted", op, rev);
    handle.onProgress((done, total) => {
      if (this.jobHandle !== handle) return;
      const cur = this.jobStore.value;
      this.jobStore.commit({
        status: "running",
        progress: { done, total },
        ...(cur.job_id !== undefined ? { job_id: cur.job_id } : {}),
      });
    });
    void handle.done.then(
      (result) => {
        if (this.jobHandle === handle) this.jobHandle = undefined;
        const r = result as { job_id?: string; stages?: JobProjection["stages"] };
        const rev = this.jobStore.commit({
          status: "succeeded",
          ...(r.job_id !== undefined ? { job_id: r.job_id } : {}),
          ...(r.stages !== undefined ? { stages: r.stages } : {}),
        });
        if (r.job_id !== undefined) this.jobContext.jobId = r.job_id;
        emit("applied", undefined, rev);
      },
      (err) => {
        if (this.jobHandle === handle) this.jobHandle = undefined;
        const cancelled = isServiceError(err) && err.code === "cancelled";
        const detail = isServiceError(err) ? err.message : String(err);
        const data = isServiceError(err) ? err.data : undefined;
        const stages =
          typeof data === "object" && data !== null && "stages" in data
            ? (data as { stages?: JobProjection["stages"] }).stages
            : undefined;
        const jobId =
          typeof data === "object" && data !== null && "job_id" in data
            ? (data as { job_id?: string }).job_id
            : undefined;
        const rev = this.jobStore.commit({
          status: cancelled ? "cancelled" : "failed",
          detail,
          ...(stages !== undefined ? { stages } : {}),
          ...(jobId !== undefined ? { job_id: jobId } : {}),
        });
        emit("rejected", cancelled ? "cancelled" : detail, rev);
      },
    );
  }

  /** `al.media.preview`: transport action → preview 状態機械。
   *  service に `al.media.*` op はまだ無い — この projection は
   *  transport intent を記録する UI-local 状態であり、実 playback
   *  backend 接続までは position_s は進まない(後続 issue 領域)。 */
  private previewIntent(
    intent: CommandIntent,
    _ctx: DispatchContext,
    emit: (status: CommandReceipt["status"], detail?: string, revision?: number | string) => void,
  ): void {
    if (intent.kind !== "action") {
      if (_ctx.expectReceipt)
        emit("unsupported", `preview target accepts actions only, got ${intent.kind}`);
      return;
    }
    const cur = this.previewStore.value;
    const apply = (state: PreviewProjection["state"], position_s = cur.position_s) => {
      emit("applied", intent.action, this.previewStore.commit({ state, position_s }));
    };
    switch (intent.action) {
      case "play":
        apply("playing");
        return;
      case "pause":
        if (cur.state === "playing") apply("paused");
        else emit("rejected", "not playing");
        return;
      case "resume":
        if (cur.state === "paused") apply("playing");
        else emit("rejected", "not paused");
        return;
      case "stop":
        apply("stopped", 0);
        return;
      default:
        emit("unsupported", `unknown preview action ${intent.action}`);
        return;
    }
  }

  /** `al.view.range`: `-lo`/`-hi` edge commit → `{low, high}` 更新。
   *  非 finite 値と `low > high` の反転は truth にしない(element は
   *  自身で clamp するが、生の push 経路からは反転し得る)。 */
  private rangeIntent(
    intent: CommandIntent,
    ctx: DispatchContext,
    emit: (status: CommandReceipt["status"], detail?: string, revision?: number | string) => void,
  ): void {
    if (intent.kind !== "commit") {
      if (ctx.expectReceipt)
        emit("unsupported", `range target accepts commits only, got ${intent.kind}`);
      return;
    }
    const plain = intent.value.plain;
    if (!Number.isFinite(plain)) {
      emit("rejected", "non-finite range edge");
      return;
    }
    const cur = this.rangeStore.value;
    const next: RangeValue = intent.controlId.endsWith("-hi")
      ? { low: cur.low, high: plain }
      : { low: plain, high: cur.high };
    if (next.low > next.high) {
      emit("rejected", `inverted range ${next.low} > ${next.high}`);
      return;
    }
    emit("applied", undefined, this.rangeStore.commit(next));
  }

  /** `al.view.*`/`al.cfg.*` scalar: commit だけが確定値を変える。
   *  config の値語彙は numeric のみ(boolean key を `1` で書くと
   *  service 側 overlay が type 不一致で捨てるため、bool 対応は
   *  実 key が必要になった時点で導入する)。 */
  private scalarIntent(
    intent: CommandIntent,
    _ctx: DispatchContext,
    emit: (status: CommandReceipt["status"], detail?: string, revision?: number | string) => void,
  ): void {
    if (intent.kind === "action") {
      emit("unsupported", `view/config target accepts gestures only, got action`);
      return;
    }
    if (intent.kind !== "commit") return; // begin/preview/cancel: 状態を変えない
    const plain = intent.value.plain;
    if (!Number.isFinite(plain)) {
      emit("rejected", "non-finite value");
      return;
    }
    if (intent.target.startsWith("al.cfg.stages.")) {
      const stage = intent.target.slice("al.cfg.stages.".length);
      if (!STAGE_SET.has(stage)) {
        emit("rejected", `unknown stage ${stage}`);
        return;
      }
      this.draft.stagesEdited = true;
      if (plain >= 0.5) this.draft.stages.add(stage);
      else this.draft.stages.delete(stage);
    } else if (intent.target.startsWith("al.cfg.")) {
      const path = intent.target.slice("al.cfg.".length);
      if (path === "" || path.split(".").some((k) => FORBIDDEN_CONFIG_KEYS.has(k))) {
        emit("rejected", `invalid config path ${path}`);
        return;
      }
      this.draft.config.set(path, plain);
    }
    const store = this.declareTarget(intent.target, plain);
    emit("applied", undefined, store.commit(plain));
  }

  // ── query 内処理 ─────────────────────────────────────────────────

  /** sourceId 文法:
   *  `waveform/<session>/<inputId>/<channel>`
   *  `spectrogram/<session>/<inputId>/<channel>[/<rows>]`
   *  `events|overview/<session>`、`features/<session>/<role>` */
  private parseSourceId(sourceId: string): SourceHandle | undefined {
    const parts = sourceId.split("/");
    if (parts.length < 2) return undefined;
    const [kind, sessionStr, a, b] = parts;
    // 数値 segment は canonical な 10 進のみ受ける — `Number()` は
    // `""`→0・`"0x1"`→1・`"3.0"`→3・`" 3 "`→3 を許し、文法の外から
    // 実在する session/channel を指せてしまうため。
    const digits = (s: string | undefined): number | undefined =>
      s !== undefined && /^\d+$/.test(s) ? Number(s) : undefined;
    const session = digits(sessionStr);
    if (session === undefined) return undefined;
    if (kind === "waveform" || kind === "spectrogram") {
      if (a === undefined || a === "") return undefined;
      // channel segment は省略可(=0)だが、存在するなら canonical な
      // 10 進に限る。負数・非整数・別記法は service が `as_u64` 失敗で
      // 0 へ黙って落とす — 別 channel の truth を返させないためここで
      // 落とす。余分な末尾 segment も文法外として拒否する。
      const channel = b === undefined ? 0 : digits(b);
      if (channel === undefined) return undefined;
      const maxSegs = kind === "waveform" ? 4 : 5;
      if (parts.length > maxSegs) return undefined;
      // spectrogram は第4セグメント `<rows>` で frequency 軸解像度を
      // view 側から固定指定できる(時間軸解像度 cols は query 側の
      // LOD budget)。正整数以外は解釈せず id 全体を malformed とする。
      const rowsSeg = parts[4];
      let rows: number | undefined;
      if (kind === "spectrogram" && rowsSeg !== undefined && rowsSeg !== "") {
        const r = digits(rowsSeg);
        if (r === undefined || r <= 0) return undefined;
        rows = r;
      }
      return rows === undefined
        ? { kind, session, inputId: a, channel }
        : { kind, session, inputId: a, channel, rows };
    }
    if (kind === "events" || kind === "overview") {
      if (parts.length !== 2) return undefined;
      return { kind, session };
    }
    if (kind === "features") {
      const role = parts.slice(2).join("/");
      return role === "" ? { kind, session } : { kind, session, role };
    }
    if (kind === "read") {
      // `read/<session>/<role...>/<name>` — role は `/` を含み得るため
      // 末尾を name、残りを role として join する。
      if (a === undefined || b === undefined) return undefined;
      const name = parts.at(-1);
      const role = parts.slice(2, -1).join("/");
      if (name === undefined || name === "" || role === "") return undefined;
      return { kind, session, role, name };
    }
    return undefined;
  }

  private resolveSource(sourceId: string): SourceHandle {
    const src = this.parseSourceId(sourceId);
    if (src === undefined) throw new Error(`malformed source id ${sourceId}`);
    if (!this.sessions.has(src.session)) {
      throw new Error(`no open dataset session ${src.session}`);
    }
    return src;
  }

  private sourceOp(
    src: SourceHandle,
    q: ViewportQuery,
  ): { op: string; params: Record<string, unknown> } {
    switch (src.kind) {
      case "waveform":
        return {
          op: AL_OPS.viewWaveform,
          params: {
            session: src.session,
            input_id: src.inputId,
            channel: src.channel ?? 0,
            start_s: q.start,
            end_s: q.end,
            bins: q.buckets ?? 1024,
          },
        };
      case "spectrogram":
        return {
          op: AL_OPS.viewSpectrogram,
          params: {
            session: src.session,
            input_id: src.inputId,
            channel: src.channel ?? 0,
            start_s: q.start,
            end_s: q.end,
            // cols は時間軸の LOD budget(query 側 `buckets`)、
            // rows は sourceId 固定の frequency 軸解像度 — 未指定は
            // service の既定(256)に委ねるため param を載せない。
            cols: q.buckets ?? 1024,
            ...(src.rows !== undefined ? { rows: src.rows } : {}),
          },
        };
      case "events":
        return {
          op: AL_OPS.dsEvents,
          params: { session: src.session, start_s: q.start, end_s: q.end },
        };
      case "features":
        return {
          op: AL_OPS.dsFeatures,
          params: {
            session: src.session,
            ...(src.role !== undefined ? { roles: [src.role] } : {}),
            limit: q.buckets ?? 512,
          },
        };
      case "overview":
        return { op: AL_OPS.dsOverview, params: { session: src.session } };
      case "read":
        return {
          op: AL_OPS.dsRead,
          params: {
            session: src.session,
            kind: "table",
            role: src.role,
            name: src.name,
            row_start: q.start,
            row_end: q.end,
          },
        };
      default:
        throw new Error(`unsupported source kind ${src.kind}`);
    }
  }

  /** op 呼出と AbortSignal の race。abort でローカル側は reject
   *  されるが service 側の呼出自体は中断できない(op に中断語彙が
   *  無い — query 層が `cancelQuery` で明示 cancel を試みる)。
   *  resolve 先着時は abort listener を外す。 */
  private callWithAbort(
    op: { op: string; params: Record<string, unknown> },
    signal?: AbortSignal,
  ): Promise<unknown> {
    const p = this.port.call(op.op, op.params);
    if (signal === undefined) return p;
    return new Promise((resolve, reject) => {
      const onAbort = (): void => reject(abortError());
      signal.addEventListener("abort", onAbort, { once: true });
      p.then(
        (v) => {
          signal.removeEventListener("abort", onAbort);
          resolve(v);
        },
        (e) => {
          signal.removeEventListener("abort", onAbort);
          reject(e);
        },
      );
    });
  }
}

/** `{low, high}` の有限数検証。 */
function isRangeValue(v: unknown): v is RangeValue {
  return (
    typeof v === "object" &&
    v !== null &&
    typeof (v as RangeValue).low === "number" &&
    Number.isFinite((v as RangeValue).low) &&
    typeof (v as RangeValue).high === "number" &&
    Number.isFinite((v as RangeValue).high) &&
    (v as RangeValue).low <= (v as RangeValue).high
  );
}

/** op 結果の転送 byte 数(`{source,bytes,meta}` publish 結果 or JSON サイズ)。 */
function replyBytes(result: unknown): number {
  if (typeof result === "object" && result !== null && "bytes" in result) {
    const b = (result as { bytes?: unknown }).bytes;
    if (typeof b === "number" && Number.isFinite(b)) return b;
  }
  return JSON.stringify(result).length;
}

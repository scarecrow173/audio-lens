/**
 * Analysis Dataset → ruviform viz data-port adapter(#132)。
 *
 * `al.view.waveform` / `al.view.spectrogram` が publish する
 * `RVBW`/`RVST` payload を、vendored ruviform `ui/viz` の bounded
 * data-port 契約(`WaveformDataSource` と同形の tile port)へ適合させる。
 *
 * 設計境界:
 * - query は `QueryBinding`(#131 gateway の sourceId grammar 経由)を
 *   通り、`generation`/`sourceRevision`/stale/cancel の意味をそのまま
 *   継承する — ここで独自の revision 語彙を作らない。
 * - bytes は `AlServicePort.pull`(`rvf://data` range-pull 相当)で
 *   page 読みし、op の descriptor(`{source, bytes, meta}`)で宣言された
 *   転送量を超えない。
 * - `NaN`/欠測は decode の値をそのまま保持し、補間・0 埋め・quantized
 *   値の逆写像を行わない。
 * - `sourceRevision` は dataset の fingerprint(再解析で変化) —
 *   publish 競合は stale として表面化し、古い generation を描かない。
 */

import type {
  QueryErrorKind,
  QueryReply,
  ViewportQuery,
} from "../../vendor/ruviform/ui/binding/src/index.ts";
import { QueryBinding } from "../../vendor/ruviform/ui/binding/src/index.ts";
import type {
  SourceMeta,
  WaveformBucket,
  WaveformDataSource,
  WaveformInfo,
  WaveformQuery,
  WaveformView,
} from "../../vendor/ruviform/ui/viz/src/index.ts";
import { decodeRvbw, decodeRvst } from "./decode.ts";

/** query port が `signal` 経由で abort されたときの rejection。 */
export function abortErr(): DOMException {
  return new DOMException("aborted", "AbortError");
}

/** QueryBinding の非 ok outcome を保持する拒否(stale/over-budget 等)。 */
export class VizQueryFailure extends Error {
  readonly kind: Exclude<QueryErrorKind, "cancelled">;
  constructor(kind: Exclude<QueryErrorKind, "cancelled">, detail: string) {
    super(detail);
    this.kind = kind;
  }
}

/** `al.view.*` が返す publish descriptor。 */
export interface AlViewDescriptor {
  readonly source: string;
  readonly bytes: number;
  readonly meta: Record<string, unknown>;
}

/** gateway 側から adapter が要求する最小 query 面。 */
export interface VizQueryGateway {
  query<T>(q: ViewportQuery, signal?: AbortSignal): Promise<QueryReply<T>>;
  cancelQuery?(queryId: string): void;
  /** source の現在 revision(dataset fingerprint)。 */
  probeRevision(sourceId: string): number | string | undefined;
}

/** `AlServicePort.pull` の型別名。 */
export type PullFn = (source: string, offset: number, limit: number) => Promise<Uint8Array>;

const DEFAULT_PULL_CHUNK = 1024 * 1024;

function isViewDescriptor(v: unknown): v is AlViewDescriptor {
  if (typeof v !== "object" || v === null) return false;
  const d = v as { source?: unknown; bytes?: unknown; meta?: unknown };
  return (
    typeof d.source === "string" &&
    typeof d.bytes === "number" &&
    Number.isFinite(d.bytes) &&
    d.bytes >= 0 &&
    typeof d.meta === "object" &&
    d.meta !== null
  );
}

async function pullAll(
  pull: PullFn,
  source: string,
  declaredBytes: number,
  signal: AbortSignal | undefined,
  chunk = DEFAULT_PULL_CHUNK,
): Promise<Uint8Array> {
  const parts: Uint8Array[] = [];
  let off = 0;
  while (off < declaredBytes) {
    if (signal?.aborted === true) throw abortErr();
    const limit = Math.min(chunk, declaredBytes - off);
    const part = await pull(source, off, limit);
    if (part.length > limit) {
      throw new VizQueryFailure(
        "error",
        `pull ${source} returned ${part.length}B > limit ${limit}B`,
      );
    }
    parts.push(part);
    off += part.length;
    if (part.length < limit) break; // short read = EOF
  }
  const out = new Uint8Array(off);
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

interface SourceArgs {
  readonly gateway: VizQueryGateway;
  readonly pull: PullFn;
  readonly session: number;
  readonly inputId: string;
  readonly pullChunkBytes?: number;
  /** request 毎の `maxBytes`(既定は QueryBinding の 16 MiB)。 */
  readonly maxBytes?: number;
}

/** `al.view.waveform` → `WaveformDataSource` の dataset adapter。 */
export class DatasetWaveformSource implements WaveformDataSource {
  private readonly binding: QueryBinding;
  private readonly gateway: VizQueryGateway;
  private readonly pull: PullFn;
  private readonly session: number;
  private readonly inputId: string;
  private readonly rate: number;
  private readonly frames: number;
  private readonly channelCount: number;
  private readonly pullChunk: number;
  private readonly maxBytes: number | undefined;

  constructor(
    args: SourceArgs & {
      readonly sampleRate: number;
      readonly frames: number;
      readonly channels: number;
    },
  ) {
    if (!(args.sampleRate > 0)) throw new Error("sampleRate must be > 0");
    if (args.channels < 1) throw new Error("channels must be >= 1");
    if (!(args.frames >= 0)) throw new Error("frames must be >= 0");
    if (!(args.pullChunkBytes === undefined || args.pullChunkBytes > 0)) {
      // pullChunk <= 0 は pullAll の short-read EOF 判定を壊して
      // 無限ループにする — 契約違反として ctor で落とす。
      throw new Error("pullChunkBytes must be > 0");
    }
    this.gateway = args.gateway;
    this.pull = args.pull;
    this.session = args.session;
    this.inputId = args.inputId;
    this.rate = args.sampleRate;
    this.frames = args.frames;
    this.channelCount = args.channels;
    this.pullChunk = args.pullChunkBytes ?? DEFAULT_PULL_CHUNK;
    this.maxBytes = args.maxBytes;
    this.binding = new QueryBinding({
      viewId: `waveform/${args.session}/${args.inputId}`,
      port: args.gateway,
      revisionProbe: (id) => args.gateway.probeRevision(id),
      // channel fan-out は 1 query 内で `channels` 個の request を
      // 並行発行する — 既定 4 だと >4ch で oldest が evict=cancel
      // されて必ず失敗するため、fan-out 分は頭数を確保する。
      maxInflight: Math.max(4, args.channels),
    });
  }

  private baseId(): string {
    return `waveform/${this.session}/${this.inputId}`;
  }

  private revision(): number | string {
    return this.gateway.probeRevision(this.baseId()) ?? 0;
  }

  meta(): WaveformInfo & SourceMeta {
    return {
      id: this.baseId(),
      revision: this.revision(),
      axis: "seconds",
      extent: this.frames / this.rate,
      channels: this.channelCount,
      sampleRate: this.rate,
    };
  }

  /** Abort → QueryBinding の cancelAll。1 source = 1 consumer
   *  (rvf element) の前提で、その query の失効はその in-flight
   *  request 群全体の cancel として伝える。 */
  private onAbort = (): void => this.binding.cancelAll();

  async query(q: WaveformQuery, signal?: AbortSignal): Promise<WaveformView> {
    if (signal?.aborted === true) throw abortErr();
    if (!Number.isFinite(q.start) || !Number.isFinite(q.end) || !(q.end > q.start)) {
      // 反転/非有限 viewport は service 側が黙って [start,start+1)
      // へ clamp する — 要求していない truth を返させない。
      throw new VizQueryFailure("error", `invalid viewport ${q.start}..${q.end}`);
    }
    if (q.buckets !== undefined && (!Number.isInteger(q.buckets) || q.buckets < 1)) {
      // 非整数 buckets は service の `as_u64` 失敗で既定 1024 bins に
      // 落ち、要求以上の truth を返す契約違反になる。
      throw new VizQueryFailure("error", `buckets must be a positive integer`);
    }
    const rev = this.revision();
    // channels 未指定/空 = 全 channel(vendored MemoryWaveformSource
    // と同規約)。範囲外 channel は黙って捨てず unsupported で
    // 表面化する — 要求した channel が返らないことを隠さない。
    const channels =
      q.channels === undefined || q.channels.length === 0
        ? Array.from({ length: this.channelCount }, (_, i) => i)
        : q.channels;
    for (const c of channels) {
      if (!Number.isInteger(c) || c < 0 || c >= this.channelCount) {
        throw new VizQueryFailure("unsupported", `channel ${c} out of range`);
      }
    }
    signal?.addEventListener("abort", this.onAbort);
    try {
      const rows = await Promise.all(
        channels.map(async (ch) => {
          const out = await this.binding.request({
            sourceId: `${this.baseId()}/${ch}`,
            sourceRevision: rev,
            start: q.start,
            end: q.end,
            buckets: q.buckets,
            channels: [ch],
            ...(this.maxBytes !== undefined ? { maxBytes: this.maxBytes } : {}),
          });
          if (!out.ok) {
            if (out.error.kind === "cancelled") throw abortErr();
            throw new VizQueryFailure(
              out.error.kind,
              out.error.detail ?? `${out.error.kind} on ${ch}`,
            );
          }
          const d = out.reply.payload;
          if (!isViewDescriptor(d)) {
            throw new VizQueryFailure(
              "error",
              `al.view.waveform descriptor malformed for ${this.baseId()}/${ch}`,
            );
          }
          const payload = await pullAll(this.pull, d.source, d.bytes, signal, this.pullChunk);
          return {
            revision: out.reply.sourceRevision,
            descriptor: d,
            view: decodeRvbw(payload),
          };
        }),
      );
      const first = rows[0];
      if (first === undefined || first.view.bins.length === 0) {
        return {
          kind: "bins",
          revision: first?.revision ?? rev,
          start: q.start,
          unitsPerBucket: q.buckets > 0 ? (q.end - q.start) / q.buckets : 1,
          channels: rows.map(() => []),
        };
      }
      // covered 範囲は descriptor meta(`LodBytes.start/end/bins` —
      // `view()` の出力範囲を encode 側が権威として返す値)から秒へ
      // 変換する。bin record の start/end は frame 単位で同一契約だが
      // meta 側が clamp 済み truth。channel 間で binning が一致する
      // ことを確認する — 共有軸を黙って揃えると別 truth を 1 axis に
      // 乗せてしまう。
      const n = first.view.bins.length;
      const m = first.descriptor.meta;
      const b0 = first.view.bins[0];
      const bN = first.view.bins[n - 1];
      if (b0 === undefined || bN === undefined) {
        throw new VizQueryFailure("error", "RVBW view lost bins after decode");
      }
      // meta.start/end が非有限(NaN 等)なら bin record の covered
      // range を使う — NaN を軸へ流さない。
      const startF = Number.isFinite(m.start) ? (m.start as number) : b0.start;
      const endF = Number.isFinite(m.end) ? (m.end as number) : bN.end;
      const startS = startF / this.rate;
      const unitsPerBucket = (endF - startF) / n / this.rate;
      for (const r of rows.slice(1)) {
        // 共有軸を暗黙に揃えない — channel 間で binning/covered
        // range が違うなら別 truth を 1 axis に乗せることになるため
        // error で表面化する。
        const rc0 = r.view.bins[0];
        const rcN = r.view.bins[r.view.bins.length - 1];
        if (r.view.bins.length !== n || rc0?.start !== b0.start || rcN?.end !== bN.end) {
          throw new VizQueryFailure(
            "error",
            `channel binning mismatch: ${r.view.bins.length} bins [${rc0?.start}..${rcN?.end}] vs ${n} [${b0.start}..${bN.end}]`,
          );
        }
      }
      return {
        kind: "bins",
        revision: first.revision,
        start: startS,
        unitsPerBucket,
        channels: rows.map((r) =>
          r.view.bins.map(
            (b): WaveformBucket => ({
              min: b.min,
              max: b.max,
              rms: b.rms,
              count: b.count,
            }),
          ),
        ),
      };
    } finally {
      signal?.removeEventListener("abort", this.onAbort);
    }
  }
}

/** `al.view.spectrogram` の tile query。`start`/`end` は秒。 */
export interface SpectrogramTileQuery {
  readonly start: number;
  readonly end: number;
  /** 時間軸(横)の LOD budget — `cols`。 */
  readonly cols: number;
  /** frequency 軸(縦)の解像度 — `rows`。 */
  readonly rows: number;
}

/**
 * decode 済み spectrogram tile。`frame f は sample f*hop 始点` の
 * STFT 規約(`DatasetSpectrum`/`FrameGrid` first_start=0)で
 * 秒・Hz の軸写像を持つ。
 */
export class SpectrogramTileView {
  readonly revision: number | string;
  readonly inputId: string;
  readonly channel: number;
  readonly frameStart: number;
  readonly frameEnd: number;
  readonly cols: number;
  readonly rows: number;
  readonly bins: number;
  readonly hop: number;
  readonly window: number;
  readonly sampleRate: number;
  readonly dtype: "f64" | "f32" | "u8";
  /** row-major `rows × cols`、row 0 = 最低 Hz。NaN / `255` sentinel 保持。 */
  readonly values: Float64Array | Float32Array | Uint8Array;

  constructor(fields: {
    revision: number | string;
    inputId: string;
    channel: number;
    frameStart: number;
    frameEnd: number;
    cols: number;
    rows: number;
    bins: number;
    hop: number;
    window: number;
    sampleRate: number;
    dtype: "f64" | "f32" | "u8";
    values: Float64Array | Float32Array | Uint8Array;
  }) {
    this.revision = fields.revision;
    this.inputId = fields.inputId;
    this.channel = fields.channel;
    this.frameStart = fields.frameStart;
    this.frameEnd = fields.frameEnd;
    this.cols = fields.cols;
    this.rows = fields.rows;
    this.bins = fields.bins;
    this.hop = fields.hop;
    this.window = fields.window;
    this.sampleRate = fields.sampleRate;
    this.dtype = fields.dtype;
    this.values = fields.values;
  }

  private framesPerCol(): number {
    return (this.frameEnd - this.frameStart) / this.cols;
  }

  private frameSeconds(f: number): number {
    return (f * this.hop) / this.sampleRate;
  }

  /** col c がカバーする source 秒の開始。 */
  colStartSeconds(c: number): number {
    return this.frameSeconds(this.frameStart + c * this.framesPerCol());
  }

  /** col c がカバーする source 秒の終端(exclusive)。 */
  colEndSeconds(c: number): number {
    return this.frameSeconds(this.frameStart + (c + 1) * this.framesPerCol());
  }

  /** row r がカバーする Hz の下端(uniform bin axis — `rate/rows` 刻み)。 */
  rowStartHz(r: number): number {
    return (r * this.sampleRate) / this.rows;
  }

  /** row r がカバーする Hz の上端(exclusive)。 */
  rowEndHz(r: number): number {
    return ((r + 1) * this.sampleRate) / this.rows;
  }
}

/** `al.view.spectrogram` → bounded tile port の dataset adapter。 */
export class DatasetSpectrogramSource {
  private readonly binding: QueryBinding;
  private readonly gateway: VizQueryGateway;
  private readonly pull: PullFn;
  private readonly session: number;
  private readonly inputId: string;
  private readonly channel: number;
  private readonly pullChunk: number;
  private readonly maxBytes: number | undefined;

  constructor(args: SourceArgs & { readonly channel: number }) {
    if (!(args.pullChunkBytes === undefined || args.pullChunkBytes > 0)) {
      throw new Error("pullChunkBytes must be > 0");
    }
    this.gateway = args.gateway;
    this.pull = args.pull;
    this.session = args.session;
    this.inputId = args.inputId;
    this.channel = args.channel;
    this.pullChunk = args.pullChunkBytes ?? DEFAULT_PULL_CHUNK;
    this.maxBytes = args.maxBytes;
    this.binding = new QueryBinding({
      viewId: `spectrogram/${args.session}/${args.inputId}/${args.channel}`,
      port: args.gateway,
      revisionProbe: (id) => args.gateway.probeRevision(id),
    });
  }

  private baseId(): string {
    return `spectrogram/${this.session}/${this.inputId}/${this.channel}`;
  }

  private revision(): number | string {
    return this.gateway.probeRevision(this.baseId()) ?? 0;
  }

  meta(): SourceMeta & { readonly inputId: string; readonly channel: number } {
    return {
      id: this.baseId(),
      revision: this.revision(),
      inputId: this.inputId,
      channel: this.channel,
    };
  }

  private onAbort = (): void => this.binding.cancelAll();

  async query(q: SpectrogramTileQuery, signal?: AbortSignal): Promise<SpectrogramTileView> {
    if (signal?.aborted === true) throw abortErr();
    if (!Number.isFinite(q.start) || !Number.isFinite(q.end) || !(q.end > q.start)) {
      throw new VizQueryFailure("error", `invalid viewport ${q.start}..${q.end}`);
    }
    if (!Number.isInteger(q.rows) || !(q.rows > 0) || !Number.isInteger(q.cols) || !(q.cols > 0)) {
      // 非整数は service 側 `as_u64` 失敗で既定値へ黙って落ちる
      // — 要求していない truth を返させない。
      throw new VizQueryFailure("error", `rows/cols must be positive integers`);
    }
    const rev = this.revision();
    signal?.addEventListener("abort", this.onAbort);
    try {
      const out = await this.binding.request({
        sourceId: `${this.baseId()}/${q.rows}`,
        sourceRevision: rev,
        start: q.start,
        end: q.end,
        buckets: q.cols,
        channels: [this.channel],
        ...(this.maxBytes !== undefined ? { maxBytes: this.maxBytes } : {}),
      });
      if (!out.ok) {
        if (out.error.kind === "cancelled") throw abortErr();
        throw new VizQueryFailure(out.error.kind, out.error.detail ?? out.error.kind);
      }
      const d = out.reply.payload;
      if (!isViewDescriptor(d)) {
        throw new VizQueryFailure(
          "error",
          `al.view.spectrogram descriptor malformed for ${this.baseId()}`,
        );
      }
      const payload = await pullAll(this.pull, d.source, d.bytes, signal, this.pullChunk);
      const t = decodeRvst(payload);
      const m = d.meta;
      const num = (k: string): number => {
        const v = m[k];
        if (typeof v !== "number" || !Number.isFinite(v)) {
          throw new VizQueryFailure("error", `spectrogram meta.${k} missing/invalid`);
        }
        return v;
      };
      const frameStart = num("frame_start");
      const frameEnd = num("frame_end");
      const hop = num("hop");
      const sampleRate = num("sample_rate");
      if (frameEnd < frameStart || hop <= 0 || sampleRate <= 0) {
        // 軸写像が成立しない descriptor — 反転/零割りの軸を
        // 黙って返さない。
        throw new VizQueryFailure(
          "error",
          `spectrogram meta axis invalid: frames ${frameStart}..${frameEnd} hop=${hop} rate=${sampleRate}`,
        );
      }
      return new SpectrogramTileView({
        revision: out.reply.sourceRevision,
        inputId: this.inputId,
        channel: this.channel,
        frameStart,
        frameEnd,
        cols: t.cols,
        rows: t.rows,
        bins: num("bins"),
        hop,
        window: num("window"),
        sampleRate,
        dtype: t.dtype === 0 ? "f64" : t.dtype === 1 ? "f32" : "u8",
        values: t.values,
      });
    } finally {
      signal?.removeEventListener("abort", this.onAbort);
    }
  }
}

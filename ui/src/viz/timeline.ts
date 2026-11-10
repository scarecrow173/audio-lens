/**
 * 同期 Timeline モデル(#132)。
 *
 * 永続 Analysis Dataset の `al.ds.overview`/`al.ds.locate`/
 * `al.ds.locate_detail`/`al.ds.events` 応答を、共有 capture-time 軸
 * (source/capture/reference を同一時間軸に並べる view model)へ
 * 写像する。`Rvf*` element の `overlays` typed property へそのまま
 * 渡せる marker 形状を提供するが、描画意味は AudioLens 側で保持する
 * (Ruviform は位置とラベルのみ描く)。
 *
 * 意味保持:
 * - alignment 行(`locate/summary`)は `status`/`validity`/`confidence`/
 *   `agreeing_methods`/`capture_start`/`capture_end` を verbatim に
 *   残し、accepted と rejected を表示情報として区別する。
 * - capture↔source の写像は `al.ds.locate_detail` の
 *   `{result, derived}` envelope 内 `normalized_source_range`/
 *   `capture_range`/`source_pcm` または `time_map` artifact
 *   (segment/anchor 補間 — Rust `TimeMap::capture_to_source` と同じ
 *   片側 interpolation)を使う。写像不能な範囲は `undefined` を返し、
 *   NaN/0 で誤魔化さない。
 *   `time_map.json` artifact は `materialize_locate` が
 *   `locate/<src>/<cap>/time_map.json` へ複写し、`al.ds.locate`
 *   の `detail_paths` から発見して `al.ds.read` kind=`json`
 *   で取得する(`timeMapPaths` helper がこの経路を担う)。
 *   `timeMaps` に path key で供給された view を使う。
 * - manifest inputs にあって audio artifact の無い input は
 *   `missing` として残し、available と混同しない。
 */

import type { OverlayState } from "../../vendor/ruviform/ui/viz/src/index.ts";

/** timeline input(track)の表示状態。 */
export type TrackState = "available" | "missing";

/** 共有軸上の 1 input(track)。 */
export interface TimelineTrack {
  readonly inputId: string;
  readonly role: string;
  readonly state: TrackState;
  readonly sampleRate?: number;
  readonly channels?: number;
  readonly frames?: number;
  readonly durationS?: number;
}

/** `locate/summary` 1 行 + detail の表示モデル(verbatim 保持)。 */
export interface TimelineAlignment {
  readonly sourceInputId: string;
  readonly captureInputId: string;
  readonly alignmentId?: string;
  /** `locate/summary` の `status`(accepted/rejected 等)をそのまま。 */
  readonly status?: string;
  readonly validity?: unknown;
  readonly confidence?: unknown;
  readonly agreeingMethods?: number;
  /** capture 側の対象範囲(capture-rate samples)。 */
  readonly captureStart?: number;
  readonly captureEnd?: number;
  /** `locate/` JSON artifact path(detail_paths の path)。 */
  readonly detailPath?: string;
  /** `al.ds.locate_detail` 応答(`{result, derived}` envelope の raw JSON)。 */
  readonly detail?: Record<string, unknown>;
  /** `time_map.json` artifact の segments(`locate/<src>/<cap>/
   *  time_map.json` を `al.ds.read` kind=`json` で取得し
   *  `timeMapPaths`/`timeMapViewFromWire` 経由で供給される)。 */
  readonly timeMap?: TimeMapView;
}

/** `time_map.json` の 1 anchor(normalized PCM 座標の対応点)。 */
export interface TimeMapAnchor {
  readonly source: number;
  readonly capture: number;
  readonly score?: number;
}

/** `al.ds.locate_detail`/`al.job` の `time_map.json` の segment。 */
export interface TimeMapSegment {
  readonly kind?: string;
  readonly anchors: readonly TimeMapAnchor[];
  readonly sourceRange?: readonly [number, number];
  readonly captureRange?: readonly [number, number];
  readonly rate?: number;
}
export interface TimeMapView {
  readonly segments: readonly TimeMapSegment[];
}

interface TrackSeed {
  role: string;
  audio?: { sampleRate: number; channels: number; frames: number; durationS: number };
}

function num(v: unknown): number | undefined {
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}

function str(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

/** `al.ds.overview`/`al.ds.locate`/`al.ds.events` 応答から共有軸
 *  Timeline を組み立てる。入力は op の生 JSON をそのまま渡す。 */
export class SyncTimeline {
  readonly tracks: readonly TimelineTrack[];
  readonly alignments: readonly TimelineAlignment[];
  /** shared capture-time axis extent(秒)。capture の最大 duration、
   *  capture が無ければ全 input の最大 duration。 */
  readonly extentS: number;

  private readonly byInput = new Map<string, TimelineTrack>();
  private readonly alignmentMap = new Map<string, TimelineAlignment>();

  constructor(args: {
    overview: unknown;
    timeline?: unknown;
    locate?: unknown;
    locateDetails?: readonly { source: string; capture: string; detail: unknown }[];
    events?: unknown;
    /** detail_paths に伴う `time_map.json` artifact(detail key = path)。 */
    timeMaps?: ReadonlyMap<string, TimeMapView>;
  }) {
    const ov = isRecord(args.overview) ? args.overview : {};
    const tl = isRecord(args.timeline) ? args.timeline : {};
    const seeds = new Map<string, TrackSeed>();

    // overview.inputs (manifest InputReference) → role
    const ovInputs = Array.isArray(ov.inputs) ? ov.inputs : [];
    for (const i of ovInputs) {
      if (!isRecord(i)) continue;
      const id = str(i.input_id);
      if (id === undefined) continue;
      seeds.set(id, { role: str(i.role) ?? "other" });
    }
    // timeline.inputs (audio artifacts) → 表示 extent/geometry
    const tlInputs = Array.isArray(tl.inputs) ? tl.inputs : [];
    for (const i of tlInputs) {
      if (!isRecord(i)) continue;
      const id = str(i.input_id);
      if (id === undefined) continue;
      const frames = num(i.frames);
      const rate = num(i.sample_rate);
      const dur = num(i.duration_s);
      const seed = seeds.get(id) ?? { role: "other" };
      seed.audio = {
        sampleRate: rate ?? 0,
        channels: num(i.channels) ?? 0,
        frames: frames ?? 0,
        durationS: dur ?? (rate !== undefined && rate > 0 ? (frames ?? 0) / rate : 0),
      };
      seeds.set(id, seed);
    }

    const tracks: TimelineTrack[] = [];
    for (const [inputId, s] of seeds) {
      const track: TimelineTrack = {
        inputId,
        role: s.role,
        state: s.audio !== undefined ? "available" : "missing",
        ...(s.audio !== undefined
          ? {
              sampleRate: s.audio.sampleRate,
              channels: s.audio.channels,
              frames: s.audio.frames,
              durationS: s.audio.durationS,
            }
          : {}),
      };
      tracks.push(track);
      this.byInput.set(inputId, track);
    }
    this.tracks = tracks;

    // shared axis extent: capture の最大 duration(capture が無ければ全 input 最大)
    const captures = tracks.filter(
      (t) => t.role === "capture_bug" || t.role === "capture_reference",
    );
    const pool = captures.length > 0 ? captures : tracks;
    this.extentS = Math.max(0, ...pool.map((t) => t.durationS ?? 0));

    // locate/summary → alignment 表示モデル
    const locate = isRecord(args.locate) ? args.locate : {};
    const rows = Array.isArray(locate.alignments) ? locate.alignments : [];
    const detailPaths = new Map<string, string>();
    if (Array.isArray(locate.detail_paths)) {
      for (const p of locate.detail_paths) {
        if (!isRecord(p)) continue;
        const path = str(p.path);
        if (path !== undefined) detailPaths.set(path, path);
      }
    }
    const detailsByKey = new Map<string, Record<string, unknown>>();
    for (const d of args.locateDetails ?? []) {
      if (isRecord(d.detail)) {
        detailsByKey.set(`${d.source}/${d.capture}`, d.detail);
      }
    }
    const alignments: TimelineAlignment[] = [];
    for (const r of rows) {
      if (!isRecord(r)) continue;
      const src = str(r.source_input_id);
      const cap = str(r.capture_input_id);
      if (src === undefined || cap === undefined) continue;
      const key = `${src}/${cap}`;
      const detail = detailsByKey.get(key);
      const alignmentId = str(r.alignment_id);
      const status = str(r.status);
      const agreeing = num(r.agreeing_methods);
      const capStart = num(r.capture_start);
      const capEnd = num(r.capture_end);
      const detailPath = detailPaths.get(`locate/${key}.json`);
      const timeMap = args.timeMaps?.get(`locate/${key}/time_map.json`);
      const a: TimelineAlignment = {
        sourceInputId: src,
        captureInputId: cap,
        ...(alignmentId !== undefined ? { alignmentId } : {}),
        ...(status !== undefined ? { status } : {}),
        ...(r.validity !== undefined ? { validity: r.validity } : {}),
        ...(r.confidence !== undefined ? { confidence: r.confidence } : {}),
        ...(agreeing !== undefined ? { agreeingMethods: agreeing } : {}),
        ...(capStart !== undefined ? { captureStart: capStart } : {}),
        ...(capEnd !== undefined ? { captureEnd: capEnd } : {}),
        ...(detailPath !== undefined ? { detailPath } : {}),
        ...(detail !== undefined ? { detail } : {}),
        ...(timeMap !== undefined ? { timeMap } : {}),
      };
      alignments.push(a);
      this.alignmentMap.set(key, a);
    }
    this.alignments = alignments;

    // events rows(verbatim)
    const ev = isRecord(args.events) ? args.events : {};
    const evRows = Array.isArray(ev.rows) ? ev.rows : [];
    this.eventRows = evRows.filter(isRecord);
  }

  /** `al.ds.events` の行(verbatim)。 */
  readonly eventRows: readonly Record<string, unknown>[];

  /** input_id で track を取得(なければ undefined)。 */
  track(inputId: string): TimelineTrack | undefined {
    return this.byInput.get(inputId);
  }

  /** (source,capture) pair で alignment を取得。 */
  alignment(sourceInputId: string, captureInputId: string): TimelineAlignment | undefined {
    return this.alignmentMap.get(`${sourceInputId}/${captureInputId}`);
  }

  /** 共有軸上の event marker(`Rvf*.overlays` へ渡せる形)。
   * `at` は capture 秒(event 行の `start_s`)。位置の無い行は
   * marker にせず eventRows 側に残す(写像不能を位置で誤魔化さない)。 */
  markers(startS = 0, endS = this.extentS): NonNullable<OverlayState["markers"]> {
    const out: { at: number; label?: string; color?: string }[] = [];
    for (const [i, row] of this.eventRows.entries()) {
      const s = num(row.start_s);
      const at = s ?? num(row.time_s);
      if (at === undefined || at < startS || at >= endS) continue;
      const label = str(row.kind) ?? str(row.name) ?? `event ${i}`;
      out.push({ at, label });
    }
    return out;
  }

  /**
   * capture 秒を alignment の source 秒へ写像する。
   * `time_map` があれば segment/anchor 補間(`TimeMap::capture_to_source`
   * と同じ片側補間)、なければ `normalized_source_range` +
   * `capture_range` の定数オフセット写像(同 rate 前提)。
   * 写像不能(対象外・情報不足)は `undefined` を返す。
   */
  captureToSourceS(a: TimelineAlignment, captureS: number): number | undefined {
    const captureTrack = this.byInput.get(a.captureInputId);
    const capRate = captureTrack?.sampleRate;
    if (capRate === undefined || capRate <= 0) return undefined;

    const capPos = captureS * capRate;
    // time_map があれば piecewise 補間(segment 未カバー → undefined)。
    // anchor.source は normalized PCM 座標(`source_coordinate_space:
    // "normalized_pcm"` — capture rate へ resample 済み)なので、
    // 割るのは source 原本 rate ではなく normalized rate(=capRate)。
    if (a.timeMap !== undefined) {
      const pos = timeMapCaptureToSource(a.timeMap, capPos);
      return pos === undefined ? undefined : pos / capRate;
    }
    // detail の ranges から定数オフセット写像
    // (normalized_source_range.start ↔ capture_range.start を対応点とする)。
    // `al.ds.locate_detail` 応答は `locate/<src>/<cap>.json` verbatim —
    // `{result: LocateResult, derived: {...}}` の envelope なので
    // `result` を unwrap する(al-locate stage.rs の persist_json)。
    const d = a.detail;
    const res = isRecord(d) && isRecord(d.result) ? d.result : d;
    const ns = isRecord(res)
      ? num(isRecord(res.normalized_source_range) ? res.normalized_source_range.start : undefined)
      : undefined;
    const cs = isRecord(res)
      ? num(isRecord(res.capture_range) ? res.capture_range.start : undefined)
      : undefined;
    const normRate =
      isRecord(res) && isRecord(res.source_pcm) ? num(res.source_pcm.sample_rate) : undefined;
    if (ns === undefined || cs === undefined || normRate === undefined || normRate <= 0) {
      return undefined;
    }
    // capture sample → normalized source sample(同率前提:normalized
    // PCM は capture rate に resample される) → source 秒 = 同じ時刻
    // (resample は時刻を保存するので norm_pos/norm_rate = source 秒)。
    const normPos = ns + (capPos - cs);
    return normPos / normRate;
  }
}

/** `al.ds.locate` 応答の `detail_paths` から `time_map.json`
 *  artifact の path を発見する。`locate/<src>/<cap>.json`
 *  (detail 文書)と `locate/<src>/<cap>/time_map.json` が混在する
 *  ため `/time_map.json` suffix で選別する。 */
export function timeMapPaths(locate: unknown): string[] {
  if (!isRecord(locate) || !Array.isArray(locate.detail_paths)) return [];
  const out: string[] = [];
  for (const p of locate.detail_paths) {
    if (!isRecord(p)) continue;
    const path = str(p.path);
    if (path?.endsWith("/time_map.json")) out.push(path);
  }
  return out;
}

/**
 * `time_map.json` artifact の serde wire 形(snake_case
 * `capture_range` / `source_range` / `anchors` / `kind` / `rate`)
 * を `TimeMapView` へ変換する。wire 名を直接 view model に置くと
 * `_range` だけが噛み合わず全 segment を skip するため、変換は
 * この一箇所に集約する。
 *
 * `al.ds.read` kind=`json` の応答は artifact 本体
 * `{time_map: <TimeMap wire>, anchors, ...}` の envelope なので、
 * 内側の `time_map` record があればそちらを TimeMap wire として
 * 扱う。bare の `{segments}` wire 形もそのまま受理する。
 * 非 record / segments 非配列は `undefined`(map 欠落扱い)。
 */
export function timeMapViewFromWire(raw: unknown): TimeMapView | undefined {
  if (!isRecord(raw)) return undefined;
  const tm = isRecord(raw.time_map) ? raw.time_map : raw;
  const rawSegs = tm.segments;
  if (!Array.isArray(rawSegs)) return undefined;
  const segments: TimeMapSegment[] = [];
  for (const rs of rawSegs) {
    if (!isRecord(rs)) return undefined;
    const cr = rs.capture_range;
    const sr = rs.source_range;
    const captureRange =
      Array.isArray(cr) && cr.length >= 2 ? ([Number(cr[0]), Number(cr[1])] as const) : undefined;
    const sourceRange =
      Array.isArray(sr) && sr.length >= 2 ? ([Number(sr[0]), Number(sr[1])] as const) : undefined;
    const anchors: TimeMapAnchor[] = [];
    if (Array.isArray(rs.anchors)) {
      for (const ra of rs.anchors) {
        if (!isRecord(ra)) return undefined;
        const source = Number(ra.source);
        const capture = Number(ra.capture);
        const score = Number(ra.score);
        if (!Number.isFinite(source) || !Number.isFinite(capture)) return undefined;
        anchors.push({ source, capture, score: Number.isFinite(score) ? score : 0 });
      }
    }
    segments.push({
      anchors,
      ...(captureRange !== undefined ? { captureRange } : {}),
      ...(sourceRange !== undefined ? { sourceRange } : {}),
      ...(typeof rs.kind === "string" ? { kind: rs.kind } : {}),
      ...(typeof rs.rate === "number" && Number.isFinite(rs.rate) ? { rate: rs.rate } : {}),
    });
  }
  return { segments };
}

/**
 * `TimeMap::capture_to_source` と同じ semantics の piecewise
 * interpolation(対象 segment の brackets anchors 間で線形、
 * `<2` anchors または非カバー → undefined)。`capture` は capture-rate
 * samples、`source` も sample 単位で返す。
 */
export function timeMapCaptureToSource(map: TimeMapView, capture: number): number | undefined {
  for (const seg of map.segments) {
    const cr = seg.captureRange;
    if (cr === undefined) continue;
    const [cs, ce] = cr;
    if (capture < cs || capture > ce) continue;
    const anchors = seg.anchors;
    if (anchors.length < 2) return undefined;
    for (let i = 0; i + 1 < anchors.length; i++) {
      const w0 = anchors[i];
      const w1 = anchors[i + 1];
      if (w0 === undefined || w1 === undefined) continue;
      if (capture < w0.capture || capture > w1.capture) continue;
      const span = w1.capture - w0.capture;
      const frac = span === 0 ? 0 : (capture - w0.capture) / span;
      return Math.round(w0.source + frac * (w1.source - w0.source));
    }
    return undefined;
  }
  return undefined;
}

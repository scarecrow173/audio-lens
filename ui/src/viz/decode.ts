/**
 * RVBW / RVST デコーダ(#132)。
 *
 * `ruviform-viz` `encode.rs` の wire format を厳密に復号する —
 * AudioLens 側で新しい語彙・補間・丸めを行わない。`f64`/`f32` の
 * IEEE `NaN` は欠測/無効値としてそのまま保持し、`u8` タイルでは
 * `NaN → 255` の wire 規約を sentinel 値として可視化側へ残す
 * (数値への再解釈は consumer の責務 — ここでは quantum 値を
 * 権威ある f64 値へ逆写像しない)。
 *
 * 破損(不正 magic/version、長さ不整合、負サイズ)は例外で失敗する
 * — 読み取り不能を黙って 0 や空にしない。
 */

/**
 * `RVBW` per-bin record(48 B)。`start`/`end`/`count` は u64、
 * `min`/`max`/`rms` は f64。upstream `encode.rs` の doc 表記は
 * `44 B` と stale だが、実 wire は `count u64` を `+40..+48` に
 * 書く 48 B record(crate 側の layout test が `rec[40..48]` を
 * 読むことで確認済み)。
 */
export interface RvbwBin {
  readonly start: number;
  readonly end: number;
  readonly min: number;
  readonly max: number;
  readonly rms: number;
  readonly count: number;
}

/** decode した `RVBW` payload(表示範囲は bin 群の `start`/`end`)。 */
export interface RvbwView {
  readonly bins: readonly RvbwBin[];
}

export const RVBW_HEADER = 16;
export const RVBW_BIN_BYTES = 48;

function fail(msg: string): never {
  throw new Error(`RVBW/RVST decode: ${msg}`);
}

/**
 * `RVBW` payload を strict に復号する。
 * 全 bin が `{start,end,min,max,rms,count}` を持つ — bin の
 * `min`/`max`/`rms` が `NaN` のまま欠測を表し、`count === 0` が
 * 全欠測 bucket を示す(規約どおり 0 埋めしない)。
 */
export function decodeRvbw(bytes: Uint8Array): RvbwView {
  if (bytes.length < RVBW_HEADER) {
    fail(`payload ${bytes.length}B < header ${RVBW_HEADER}B`);
  }
  const magic = String.fromCharCode(bytes[0] ?? 0, bytes[1] ?? 0, bytes[2] ?? 0, bytes[3] ?? 0);
  if (magic !== "RVBW") fail(`magic ${magic} !== RVBW`);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.length);
  if (view.getUint16(4, true) !== 1) fail(`version ${view.getUint16(4, true)} !== 1`);
  const n = view.getUint32(8, true);
  const expected = RVBW_HEADER + n * RVBW_BIN_BYTES;
  if (bytes.length !== expected) {
    fail(`length ${bytes.length}B !== ${expected}B (bins=${n})`);
  }
  const bins: RvbwBin[] = new Array(n);
  for (let i = 0; i < n; i++) {
    const o = RVBW_HEADER + i * RVBW_BIN_BYTES;
    bins[i] = {
      start: Number(view.getBigUint64(o, true)),
      end: Number(view.getBigUint64(o + 8, true)),
      min: view.getFloat64(o + 16, true),
      max: view.getFloat64(o + 24, true),
      rms: view.getFloat64(o + 32, true),
      count: Number(view.getBigUint64(o + 40, true)),
    };
  }
  return { bins };
}

/** `RVST` dtype — `Quantization::F64/F32/U8` の 0/1/2。 */
export type RvstDtype = 0 | 1 | 2;

/** decode した `RVST` tile(row-major `rows × cols`、row 0 = 最低 Hz)。 */
export interface RvstTile {
  readonly dtype: RvstDtype;
  readonly cols: number;
  readonly rows: number;
  /** Tile がカバーする最初の STFT frame(`frame_start`)。 */
  readonly frameStart: number;
  /**
   * row-major の magnitude 列。`f64`/`f32` は IEEE NaN 保持、
   * `u8` は wire の `255` をそのまま残す(再正規化しない)。
   */
  readonly values: Float64Array | Float32Array | Uint8Array;
}

export const RVST_HEADER = 24;

const DTYPE_BYTES: Record<RvstDtype, number> = { 0: 8, 1: 4, 2: 1 };

/**
 * `RVST` payload を strict に復号する。値領域は payload 内の
 * little-endian buffer への view(zero-copy)を返す — `f32`/`f64` の
 * NaN payload はそのまま透過する。
 */
export function decodeRvst(bytes: Uint8Array): RvstTile {
  if (bytes.length < RVST_HEADER) {
    fail(`payload ${bytes.length}B < header ${RVST_HEADER}B`);
  }
  const magic = String.fromCharCode(bytes[0] ?? 0, bytes[1] ?? 0, bytes[2] ?? 0, bytes[3] ?? 0);
  if (magic !== "RVST") fail(`magic ${magic} !== RVST`);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.length);
  if (view.getUint16(4, true) !== 1) fail(`version ${view.getUint16(4, true)} !== 1`);
  const dtypeRaw = view.getUint16(6, true);
  if (dtypeRaw !== 0 && dtypeRaw !== 1 && dtypeRaw !== 2) {
    fail(`dtype ${dtypeRaw} unknown`);
  }
  const dtype = dtypeRaw as RvstDtype;
  const cols = view.getUint32(8, true);
  const rows = view.getUint32(12, true);
  const frameStart = Number(view.getBigUint64(16, true));
  const dsize = DTYPE_BYTES[dtype];
  const expected = RVST_HEADER + cols * rows * dsize;
  if (bytes.length !== expected) {
    fail(`length ${bytes.length}B !== ${expected}B (${rows}x${cols}@f${dsize * 8})`);
  }
  const region =
    (bytes.byteOffset + RVST_HEADER) % dsize === 0
      ? bytes.subarray(RVST_HEADER)
      : // subarray の byteOffset が dtype 境界に揃わない場合は
        // TypedArray view を作れないため、先に aligned copy を取る
        // (pull 経路は offset 0 だが decoder 単体利用を壊さない)。
        bytes.slice(RVST_HEADER);
  const values =
    dtype === 0
      ? new Float64Array(region.buffer, region.byteOffset, cols * rows)
      : dtype === 1
        ? new Float32Array(region.buffer, region.byteOffset, cols * rows)
        : region;
  return { dtype, cols, rows, frameStart, values };
}

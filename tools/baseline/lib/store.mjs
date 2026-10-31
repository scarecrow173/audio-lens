// al-dataset の公開 store レイアウトを読み取る。
// - columnar table: schema.json + index.json + <col>/<NNNNNN>.bin
//   (NULL bitmap ceil(rows/8) LSB-first + 非 null 値列。
//    i64/f64 は little-endian、utf8 は u32le len + UTF-8)
// - chunked tensor: array.json(dtype/shape/chunk_shape) +
//   chunks/<coords>.bin(f64le row-major、edge chunk は実 extent)
// - wav: RIFF PCM / IEEE-float
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { readJson } from "./common.mjs";

export async function isTableDir(dir) {
  try {
    await readJson(path.join(dir, "schema.json"));
    await readJson(path.join(dir, "index.json"));
    return true;
  } catch {
    return false;
  }
}

export async function readTable(dir) {
  const schema = await readJson(path.join(dir, "schema.json"));
  const index = await readJson(path.join(dir, "index.json"));
  const rows = index.rows ?? index.row_count ?? 0;
  const chunkRows = index.chunk_rows ?? rows;
  const nChunks = rows === 0 ? 0 : Math.ceil(rows / chunkRows);
  const columns = schema.columns ?? schema;
  const decoded = {};
  for (const col of columns) {
    const name = col.name;
    const type = col.data_type ?? col.type;
    const cells = new Array(rows).fill(null);
    const colDir = path.join(dir, name);
    for (let c = 0; c < nChunks; c++) {
      const file = path.join(colDir, `${String(c).padStart(6, "0")}.bin`);
      const buf = await readFile(file);
      const chunkStart = c * chunkRows;
      const chunkLen = Math.min(chunkRows, rows - chunkStart);
      const bitmapBytes = Math.ceil(chunkLen / 8);
      let off = bitmapBytes;
      for (let r = 0; r < chunkLen; r++) {
        const isNull = (buf[r >> 3] >> (r & 7)) & 1;
        if (isNull) continue;
        const rowIdx = chunkStart + r;
        if (type === "i64" || type === "I64") {
          cells[rowIdx] = buf.readBigInt64LE(off);
          off += 8;
        } else if (type === "f64" || type === "F64") {
          cells[rowIdx] = buf.readDoubleLE(off);
          off += 8;
        } else {
          const len = buf.readUInt32LE(off);
          off += 4;
          cells[rowIdx] = buf.toString("utf8", off, off + len);
          off += len;
        }
      }
    }
    decoded[name] = { type, cells };
  }
  return { rows, columns: decoded, schema };
}

// table を JSON 化可能な行配列へ(BigInt は文字列化 + タグ)。
export function tableToRows(table) {
  const names = Object.keys(table.columns);
  const out = [];
  for (let r = 0; r < table.rows; r++) {
    const row = {};
    for (const n of names) {
      const v = table.columns[n].cells[r];
      row[n] = typeof v === "bigint" ? { $i64: v.toString() } : v;
    }
    out.push(row);
  }
  return out;
}

export async function isTensorDir(dir) {
  try {
    await readJson(path.join(dir, "array.json"));
    return true;
  } catch {
    return false;
  }
}

export async function readTensor(dir) {
  const meta = await readJson(path.join(dir, "array.json"));
  if (meta.dtype !== "f64") {
    throw new Error(`unsupported tensor dtype: ${meta.dtype}`);
  }
  const { shape, chunk_shape: chunkShape } = meta;
  const ndim = shape.length;
  const total = shape.reduce((a, b) => a * b, 1);
  const data = new Float64Array(total);
  const chunkDir = path.join(dir, "chunks");
  const files = await readdir(chunkDir);
  const stride = new Array(ndim);
  stride[ndim - 1] = 1;
  for (let d = ndim - 2; d >= 0; d--) stride[d] = stride[d + 1] * shape[d + 1];
  for (const f of files) {
    if (!f.endsWith(".bin")) continue;
    const coords = f
      .slice(0, -4)
      .split("_")
      .map((s) => Number(s));
    const buf = await readFile(path.join(chunkDir, f));
    const extent = coords.map((c, d) => Math.min(chunkShape[d], shape[d] - c * chunkShape[d]));
    const chunkLen = extent.reduce((a, b) => a * b, 1);
    if (buf.length !== chunkLen * 8) {
      throw new Error(`tensor chunk ${f}: expected ${chunkLen * 8} bytes, got ${buf.length}`);
    }
    // chunk 内 row-major を全 tensor row-major へ scatter
    const cursor = new Array(ndim).fill(0);
    for (let i = 0; i < chunkLen; i++) {
      let off = 0;
      for (let d = 0; d < ndim; d++) {
        off += (coords[d] * chunkShape[d] + cursor[d]) * stride[d];
      }
      data[off] = buf.readDoubleLE(i * 8);
      for (let d = ndim - 1; d >= 0; d--) {
        if (++cursor[d] < extent[d]) break;
        cursor[d] = 0;
      }
    }
  }
  return { meta, data };
}

export async function findTablesAndTensors(root, walk) {
  const files = await walk(root);
  const tableRoots = new Set();
  const tensorRoots = new Set();
  for (const rel of files) {
    if (rel.endsWith("/schema.json")) tableRoots.add(rel.slice(0, -12));
    if (rel.endsWith("/array.json")) tensorRoots.add(rel.slice(0, -11));
  }
  return {
    tables: [...tableRoots].sort(),
    tensors: [...tensorRoots].sort(),
  };
}

// WAV (RIFF) を decode。PCM int / IEEE float 対応。
export async function readWav(file) {
  const buf = await readFile(file);
  if (buf.toString("ascii", 0, 4) !== "RIFF" || buf.toString("ascii", 8, 12) !== "WAVE") {
    throw new Error("not a RIFF/WAVE file");
  }
  let off = 12;
  let fmt = null;
  let dataOff = 0;
  let dataLen = 0;
  while (off + 8 <= buf.length) {
    const id = buf.toString("ascii", off, off + 4);
    const size = buf.readUInt32LE(off + 4);
    if (id === "fmt ") {
      fmt = {
        format: buf.readUInt16LE(off + 8),
        channels: buf.readUInt16LE(off + 10),
        sampleRate: buf.readUInt32LE(off + 12),
        bits: buf.readUInt16LE(off + 22),
      };
    } else if (id === "data") {
      dataOff = off + 8;
      dataLen = size;
    }
    off += 8 + size + (size & 1);
  }
  if (!fmt || !dataLen) throw new Error("wav: missing fmt or data chunk");
  const bytesPer = fmt.bits >> 3;
  const frames = dataLen / (bytesPer * fmt.channels);
  const samples = new Float64Array(frames * fmt.channels);
  for (let i = 0; i < samples.length; i++) {
    const o = dataOff + i * bytesPer;
    if (fmt.format === 3) {
      samples[i] = fmt.bits === 32 ? buf.readFloatLE(o) : buf.readDoubleLE(o);
    } else {
      samples[i] =
        fmt.bits === 16
          ? buf.readInt16LE(o) / 32768
          : fmt.bits === 24
            ? buf.readIntLE(o, 3) / 8388608
            : fmt.bits === 32
              ? buf.readInt32LE(o) / 2147483648
              : buf.readInt8(o) / 128;
    }
  }
  return { fmt, samples };
}

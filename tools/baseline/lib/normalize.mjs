// ADR「正規化契約」の実装。run グローバルな ID canonical mapping を
// manifest 登録順 → raw path 昇順 × DFS 出現順で採番し、文字列値と
// ファイル path の両方に部分文字列置換で適用する。
import path from "node:path";
import { ID_RE, readJson, walkFiles } from "./common.mjs";

function collectIdsFromString(s, record) {
  for (const m of s.matchAll(ID_RE)) record(m[0]);
}

function collectIdsFromValue(v, record) {
  if (typeof v === "string") {
    collectIdsFromString(v, record);
  } else if (Array.isArray(v)) {
    for (const x of v) collectIdsFromValue(x, record);
  } else if (v && typeof v === "object") {
    for (const k of Object.keys(v)) {
      collectIdsFromString(k, record);
      collectIdsFromValue(v[k], record);
    }
  }
}

// jobDir: run の job ディレクトリ絶対 path
// opts.absPaths: { placeholder: absolutePath } — 値に含まれた場合
//   placeholder に置換する(長い方から適用)
export async function buildRunNormalizer(jobDir, opts = {}) {
  const rawToCanon = new Map();
  const counters = new Map();
  const record = (raw) => {
    if (rawToCanon.has(raw)) return;
    const prefix = raw.slice(0, raw.indexOf("-"));
    const n = (counters.get(prefix) ?? 0) + 1;
    counters.set(prefix, n);
    rawToCanon.set(raw, `${prefix}-#${n}`);
  };

  // pass 1a: manifest.json の登録順(pipeline 決定順)
  try {
    const manifest = await readJson(path.join(jobDir, "manifest.json"));
    collectIdsFromValue(manifest, record);
  } catch {
    // manifest 不在時は path 走査のみにフォールバック
  }

  // pass 1b: raw path 昇順 × ファイル内 DFS(file path 自体も走査)
  const files = await walkFiles(jobDir);
  for (const rel of files) {
    collectIdsFromString(rel.split(path.sep).join("/"), record);
    if (rel.endsWith(".json")) {
      try {
        collectIdsFromValue(await readJson(path.join(jobDir, rel)), record);
      } catch {
        // JSON parse 不可は正規化対象外として扱う
      }
    }
  }

  // 置換は長さ降順(前方一致衝突の回避)
  const sortedRaw = [...rawToCanon.keys()].sort((a, b) => b.length - a.length);
  const absEntries = Object.entries(opts.absPaths ?? {}).sort((a, b) => b[1].length - a[1].length);

  const replaceIds = (s) => {
    let out = s;
    for (const raw of sortedRaw) {
      if (out.includes(raw)) out = out.split(raw).join(rawToCanon.get(raw));
    }
    return out;
  };

  const replaceAbs = (s) => {
    let out = s;
    for (const [ph, abs] of absEntries) {
      const fwd = abs.split(path.sep).join("/");
      if (out.includes(fwd)) out = out.split(fwd).join(ph);
      if (out.includes(abs)) out = out.split(abs).join(ph);
    }
    return out;
  };

  const applyString = (s) => replaceAbs(replaceIds(s));
  const applyPath = (s) => replaceAbs(replaceIds(s.split("\\").join("/")));

  const normalizeValue = (v) => {
    if (typeof v === "string") return applyPath(v);
    if (Array.isArray(v)) return v.map(normalizeValue);
    if (v && typeof v === "object") {
      const o = {};
      for (const k of Object.keys(v)) o[applyString(k)] = normalizeValue(v[k]);
      return o;
    }
    return v;
  };

  return {
    normalizePath: (rel) => applyPath(rel.split(path.sep).join("/")),
    normalizeValue,
    applyString,
    applyPath,
    mapping: Object.fromEntries([...rawToCanon.entries()].sort((a, b) => a[1].localeCompare(b[1]))),
  };
}

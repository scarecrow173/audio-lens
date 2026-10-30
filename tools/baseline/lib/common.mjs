// baseline ツール共通ヘルパー。外部依存なし(Node stdlib のみ)。
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

export const ID_RE = /(?<![0-9A-Za-z])(JOB|ART|IN|ALN)-[0-9A-Za-z]{22}(?![0-9A-Za-z])/g;

export function sha256File(file) {
  return new Promise((resolve, reject) => {
    const h = createHash("sha256");
    createReadStream(file)
      .on("data", (d) => h.update(d))
      .on("end", () => resolve(h.digest("hex")))
      .on("error", reject);
  });
}

export function sha256Buf(buf) {
  return createHash("sha256").update(buf).digest("hex");
}

// dir 配下の全ファイルを POSIX 形式の相対 path 昇順で返す。
export async function walkFiles(root) {
  const out = [];
  async function rec(dir) {
    for (const e of await readdir(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) await rec(p);
      else if (e.isFile()) out.push(p);
    }
  }
  await rec(root);
  return out.map((p) => path.relative(root, p).split(path.sep).join("/")).sort();
}

export async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"));
}

export async function writeJson(file, value) {
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
}

export async function fileSize(file) {
  return (await stat(file)).size;
}

// JSON 値の DFS 走査(callback は各 leaf・各 key の順に呼ばれる)。
export function* dfsLeaves(value, keyPath = []) {
  if (value === null || typeof value !== "object") {
    yield { path: keyPath, value };
    return;
  }
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) {
      yield* dfsLeaves(value[i], [...keyPath, i]);
    }
    return;
  }
  for (const k of Object.keys(value)) {
    yield* dfsLeaves(value[k], [...keyPath, k]);
  }
}

/// `**`/`/` 対応の単純 glob。run 相対パス(正規化済み `/` 区切り)にマッチ。
export function globMatch(glob, rel) {
  const re = new RegExp(
    "^" +
      glob
        .split("/")
        .map((seg) =>
          seg === "**"
            ? "(?:[^/]+/)*[^/]*"
            : seg.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\\\*/g, "[^/]*"),
        )
        .join("/") +
      "$",
  );
  return re.test(rel);
}

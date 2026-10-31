// tools/baseline/compare.mjs
// 2 つの baseline run(baseline/runs/<name>)を contracts.json の
// 意味論で比較する。差分は列挙し exit code で有無を返す。
// 使い方:
//   node tools/baseline/compare.mjs <runA> <runB> [--json <out>]
//   node tools/baseline/compare.mjs --self-test

import { existsSync } from "node:fs";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { globMatch, readJson, writeJson } from "./lib/common.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const BASELINE = path.join(ROOT, "baseline");

function isRecordOnlyField(keyPath, contracts) {
  const key = keyPath[keyPath.length - 1];
  for (const f of contracts.recordOnlyFields ?? []) {
    if (typeof f === "string" && f === key) return true;
    if (typeof f === "object" && globMatch(f.glob, keyPath.join("/"))) {
      return true;
    }
  }
  return false;
}

function toleranceFor(relPath, contracts) {
  for (const o of contracts.toleranceOverrides ?? []) {
    if (globMatch(o.glob, relPath)) return { rel: o.rel, abs: o.abs };
  }
  return contracts.defaultTolerance ?? { rel: 1e-9, abs: 1e-12 };
}

// golden JSON 構造比較。diffs に追記する。
function compareJson(a, b, relPath, keyPath, contracts, diffs) {
  if (isRecordOnlyField(keyPath, contracts)) return;
  const ta = a === null ? "null" : Array.isArray(a) ? "array" : typeof a;
  const tb = b === null ? "null" : Array.isArray(b) ? "array" : typeof b;
  if (ta !== tb) {
    diffs.push({ surface: relPath, path: keyPath.join("/"), kind: "type", a: ta, b: tb });
    return;
  }
  if (a === null) return; // null == null(非有限 or 欠損の潰れは payload 面で担保)
  if (ta === "object") {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    for (const k of [...keys].sort()) {
      if (!(k in a) || !(k in b)) {
        diffs.push({
          surface: relPath,
          path: [...keyPath, k].join("/"),
          kind: "key",
          a: k in a,
          b: k in b,
        });
        continue;
      }
      compareJson(a[k], b[k], relPath, [...keyPath, k], contracts, diffs);
    }
    return;
  }
  if (ta === "array") {
    if (a.length !== b.length) {
      diffs.push({
        surface: relPath,
        path: keyPath.join("/"),
        kind: "length",
        a: a.length,
        b: b.length,
      });
      return;
    }
    for (let i = 0; i < a.length; i++) {
      compareJson(a[i], b[i], relPath, [...keyPath, i], contracts, diffs);
    }
    return;
  }
  if (typeof a === "number") {
    const { rel, abs } = toleranceFor(`${relPath}#${keyPath.join("/")}`, contracts);
    const intA = Number.isInteger(a);
    const intB = Number.isInteger(b);
    if (intA && intB) {
      if (a !== b) {
        diffs.push({ surface: relPath, path: keyPath.join("/"), kind: "int", a, b });
      }
      if (!Number.isSafeInteger(a) || !Number.isSafeInteger(b)) {
        diffs.push({
          surface: relPath,
          path: keyPath.join("/"),
          kind: "unsafe-int",
          a,
          b,
          note: "2^53 超過: comparator の整数精度では判定不能",
        });
      }
      return;
    }
    if (!Number.isFinite(a) || !Number.isFinite(b)) {
      if (!(Number.isNaN(a) && Number.isNaN(b)) && a !== b) {
        diffs.push({
          surface: relPath,
          path: keyPath.join("/"),
          kind: "nonfinite",
          a: String(a),
          b: String(b),
        });
      }
      return;
    }
    const ok = Math.abs(a - b) <= Math.max(abs, rel * Math.max(Math.abs(a), Math.abs(b)));
    if (!ok) {
      diffs.push({ surface: relPath, path: keyPath.join("/"), kind: "tolerance", a, b, rel, abs });
    }
    return;
  }
  if (a !== b) {
    diffs.push({ surface: relPath, path: keyPath.join("/"), kind: "value", a, b });
  }
}

async function listJsonFiles(dir) {
  const out = [];
  async function rec(d, prefix) {
    for (const e of await readdir(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      const rel = prefix ? `${prefix}/${e.name}` : e.name;
      if (e.isDirectory()) await rec(p, rel);
      else if (e.isFile() && e.name.endsWith(".json")) out.push(rel);
    }
  }
  if (existsSync(dir)) await rec(dir, "");
  return out.sort();
}

async function compareRuns(nameA, nameB, contracts) {
  const dirA = path.join(BASELINE, "runs", nameA);
  const dirB = path.join(BASELINE, "runs", nameB);
  const diffs = [];

  // 1) artifacts.index.json: path 集合 + class 別規則
  const ixA = await readJson(path.join(dirA, "artifacts.index.json"));
  const ixB = await readJson(path.join(dirB, "artifacts.index.json"));
  for (const p of new Set([...Object.keys(ixA), ...Object.keys(ixB)])) {
    const ea = ixA[p];
    const eb = ixB[p];
    if (!ea || !eb) {
      diffs.push({ surface: "artifacts.index", path: p, kind: "path-set", a: !!ea, b: !!eb });
      continue;
    }
    const cls = ea.class === eb.class ? ea.class : "record-only";
    if (ea.class !== eb.class) {
      diffs.push({ surface: "artifacts.index", path: p, kind: "class", a: ea.class, b: eb.class });
    }
    if (cls === "record-only") continue;
    if (ea.size !== eb.size) {
      diffs.push({ surface: "artifacts.index", path: p, kind: "size", a: ea.size, b: eb.size });
    }
    if (cls === "bit-exact" && ea.sha256 !== eb.sha256) {
      diffs.push({
        surface: "artifacts.index",
        path: p,
        kind: "sha256",
        a: ea.sha256,
        b: eb.sha256,
      });
    }
  }

  // 2) manifest.normalized.json
  const mA = await readJson(path.join(dirA, "manifest.normalized.json"));
  const mB = await readJson(path.join(dirB, "manifest.normalized.json"));
  compareJson(mA, mB, "manifest.normalized.json", [], contracts, diffs);

  // 3) golden/ ツリー
  const gA = await listJsonFiles(path.join(dirA, "golden"));
  const gB = await listJsonFiles(path.join(dirB, "golden"));
  for (const rel of new Set([...gA, ...gB])) {
    const fa = path.join(dirA, "golden", rel);
    const fb = path.join(dirB, "golden", rel);
    const skipSurface = (contracts.recordOnlySurfaces ?? []).some((g) => globMatch(g, rel));
    if (skipSurface) continue;
    if (!existsSync(fa) || !existsSync(fb)) {
      diffs.push({
        surface: `golden/${rel}`,
        path: "",
        kind: "file-set",
        a: existsSync(fa),
        b: existsSync(fb),
      });
      continue;
    }
    compareJson(await readJson(fa), await readJson(fb), `golden/${rel}`, [], contracts, diffs);
  }
  return diffs;
}

// ---------- self-test ----------

async function selfTest(contracts) {
  const tmp = await mkdtemp(path.join(os.tmpdir(), "baseline-selftest-"));
  try {
    const a = path.join(tmp, "a");
    const b = path.join(tmp, "b");
    const mk = async (dir, mut) => {
      const ix = { "x.f64le": { size: 8, sha256: "aa", class: "bit-exact" } };
      const man = { inputs: [{ role: "capture_bug", duration_ms: 12 }], score: 1.5, n: 4 };
      const gold = { v: [1.0, 2.0], s: "ok", flag: true };
      await writeJson(path.join(dir, "artifacts.index.json"), mut.index ? mut.index(ix) : ix);
      await writeJson(
        path.join(dir, "manifest.normalized.json"),
        mut.manifest ? mut.manifest(man) : man,
      );
      await writeJson(path.join(dir, "golden", "g.json"), mut.golden ? mut.golden(gold) : gold);
    };
    const runs = [
      ["int diff", { manifest: (m) => ({ ...m, n: 5 }) }, true],
      ["tolerance ok", { manifest: (m) => ({ ...m, score: 1.5 * (1 + 1e-10) }) }, false],
      ["tolerance over", { manifest: (m) => ({ ...m, score: 1.6 }) }, true],
      [
        "record-only skip",
        { manifest: (m) => ({ ...m, inputs: [{ ...m.inputs[0], duration_ms: 999 }] }) },
        false,
      ],
      ["sha256 diff", { index: (i) => ({ "x.f64le": { ...i["x.f64le"], sha256: "bb" } }) }, true],
      ["path missing", { index: () => ({}) }, true],
    ];
    const results = [];
    for (const [name, mut, expectDiff] of runs) {
      await mk(a, {});
      await mk(b, mut);
      const diffs = await compareRunsTo(a, b, contracts);
      const got = diffs.length > 0;
      results.push({
        case: name,
        expect_diff: expectDiff,
        got_diff: got,
        pass: got === expectDiff,
        diffs,
      });
    }
    return results;
  } finally {
    await rm(tmp, { recursive: true, force: true });
  }
}

// compareRuns と同じだが任意ディレクトリを比較する(self-test 用)
async function compareRunsTo(dirA, dirB, contracts) {
  const diffs = [];
  const ixA = await readJson(path.join(dirA, "artifacts.index.json"));
  const ixB = await readJson(path.join(dirB, "artifacts.index.json"));
  for (const p of new Set([...Object.keys(ixA), ...Object.keys(ixB)])) {
    const ea = ixA[p];
    const eb = ixB[p];
    if (!ea || !eb) {
      diffs.push({ surface: "artifacts.index", path: p, kind: "path-set", a: !!ea, b: !!eb });
      continue;
    }
    const cls = ea.class;
    if (cls === "record-only") continue;
    if (ea.size !== eb.size)
      diffs.push({ surface: "artifacts.index", path: p, kind: "size", a: ea.size, b: eb.size });
    if (cls === "bit-exact" && ea.sha256 !== eb.sha256) {
      diffs.push({
        surface: "artifacts.index",
        path: p,
        kind: "sha256",
        a: ea.sha256,
        b: eb.sha256,
      });
    }
  }
  compareJson(
    await readJson(path.join(dirA, "manifest.normalized.json")),
    await readJson(path.join(dirB, "manifest.normalized.json")),
    "manifest.normalized.json",
    [],
    contracts,
    diffs,
  );
  const gA = await listJsonFiles(path.join(dirA, "golden"));
  const gB = await listJsonFiles(path.join(dirB, "golden"));
  for (const rel of new Set([...gA, ...gB])) {
    const fa = path.join(dirA, "golden", rel);
    const fb = path.join(dirB, "golden", rel);
    if (!existsSync(fa) || !existsSync(fb)) {
      diffs.push({
        surface: `golden/${rel}`,
        path: "",
        kind: "file-set",
        a: existsSync(fa),
        b: existsSync(fb),
      });
      continue;
    }
    compareJson(await readJson(fa), await readJson(fb), `golden/${rel}`, [], contracts, diffs);
  }
  return diffs;
}

async function main() {
  const args = process.argv.slice(2);
  const contracts = await readJson(path.join(BASELINE, "contracts.json"));
  if (args[0] === "--self-test") {
    const results = await selfTest(contracts);
    const failed = results.filter((r) => !r.pass);
    console.log(
      JSON.stringify(
        results.map((r) => ({ case: r.case, pass: r.pass })),
        null,
        2,
      ),
    );
    if (failed.length) {
      console.error("self-test FAILED", JSON.stringify(failed, null, 2));
      process.exit(1);
    }
    console.log("self-test: all cases pass");
    return;
  }
  const [nameA, nameB] = args;
  if (!nameA || !nameB) {
    console.error("usage: compare.mjs <runA> <runB> [--json <out>]");
    process.exit(2);
  }
  const diffs = await compareRuns(nameA, nameB, contracts);
  const jsonIdx = args.indexOf("--json");
  if (jsonIdx >= 0) await writeJson(path.resolve(args[jsonIdx + 1]), diffs);
  console.log(`${diffs.length} diffs`);
  for (const d of diffs.slice(0, 50)) console.log(JSON.stringify(d));
  process.exit(diffs.length ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

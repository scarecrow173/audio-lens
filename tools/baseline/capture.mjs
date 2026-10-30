// tools/baseline/capture.mjs
// run matrix の実行・収集・記録オーケストレーション。
// 使い方:
//   node tools/baseline/capture.mjs [--run <name>] [--skip-runs]
//     [--collect-only <name>:<jobDir>]
// 前提: target/release/al ビルド済み・ffmpeg が PATH 上。
import { execFileSync, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { cp, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { fileSize, globMatch, readJson, sha256File, walkFiles, writeJson } from "./lib/common.mjs";
import { buildRunNormalizer } from "./lib/normalize.mjs";
import { monitorProcess } from "./lib/procmon.mjs";
import { findTablesAndTensors, readTable, readTensor, readWav, tableToRows } from "./lib/store.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const BASELINE = path.join(ROOT, "baseline");
const RUNS_OUT = path.join(BASELINE, "runs");
const TMP_RUNS = path.join(ROOT, "samples", "_runs", "baseline");
const SMALL_PAYLOAD_MAX = 65536;

function sh(cmd, args, opts = {}) {
  return execFileSync(cmd, args, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...opts,
  }).trim();
}

function trySh(cmd, args) {
  try {
    return sh(cmd, args);
  } catch {
    return null;
  }
}

// ---------- 環境・入力記録 ----------

async function writeEnvironment(cfg) {
  const dirty = trySh("git", ["status", "--porcelain"]);
  const env = {
    recorded_at: new Date().toISOString(),
    commit: trySh("git", ["rev-parse", "HEAD"]),
    branch: trySh("git", ["rev-parse", "--abbrev-ref", "HEAD"]),
    worktree_dirty: dirty == null ? null : dirty.length > 0,
    rust: {
      rustc: trySh("mise", ["exec", "--", "rustc", "--version"]),
      cargo: trySh("mise", ["exec", "--", "cargo", "--version"]),
    },
    mise_tools: cfg.miseTools ?? {},
    ffmpeg: {
      version: trySh("ffmpeg", ["-version"])?.split("\n")[0] ?? null,
      path: trySh(process.platform === "win32" ? "where" : "which", ["ffmpeg"]),
      note: "repo toolchain 外。派生 AVI 生成のみに使用(比較契約外)",
    },
    os: `${os.type()} ${os.release()} ${os.arch()}`,
    cpu: {
      model: os.cpus()[0]?.model ?? null,
      cores: os.cpus().length,
      note: "al-math は pulp 実行時 SIMD dispatch のため bit-exact 契約は同一 CPU 機能レベルに限定",
    },
    build_profile: "release (debug=true)",
    node: process.version,
  };
  await writeJson(path.join(BASELINE, "environment.json"), env);
}

async function ensureDerivedInputs(cfg) {
  for (const [name, d] of Object.entries(cfg.derivedInputs ?? {})) {
    const out = path.join(ROOT, d.output);
    if (!existsSync(out)) {
      await mkdir(path.dirname(out), { recursive: true });
      const args = d.args.map((a) =>
        a === "{in}" ? path.join(ROOT, d.source) : a === "{out}" ? out : a,
      );
      console.log(`[derive] ${name}: ffmpeg ${args.join(" ")}`);
      execFileSync("ffmpeg", args, { cwd: ROOT, stdio: "inherit" });
    }
  }
}

async function writeInputs(cfg) {
  const entries = [];
  for (const [name, spec] of Object.entries(cfg.inputs)) {
    const rel = spec.derived ? cfg.derivedInputs[spec.derived].output : spec.path;
    const abs = path.join(ROOT, rel);
    const entry = {
      name,
      role: spec.role ?? null,
      path: rel.split(path.sep).join("/"),
      origin: spec.derived
        ? {
            kind: "derived",
            source: cfg.derivedInputs[spec.derived].source,
            command: cfg.derivedInputs[spec.derived].args.join(" "),
            note: "コンテナ bytes は ffmpeg build 依存。sha256 は観測値(契約外)",
          }
        : { kind: "lfs-sample" },
      size: await fileSize(abs),
      sha256: await sha256File(abs),
    };
    entries.push(entry);
  }
  // samples/ 全実サンプルの sha256 一覧も併記
  const sampleList = [];
  for (const rel of await walkFiles(path.join(ROOT, "samples"))) {
    if (rel.startsWith("_runs/")) continue;
    const abs = path.join(ROOT, "samples", rel);
    sampleList.push({
      path: `samples/${rel}`,
      size: await fileSize(abs),
      sha256: await sha256File(abs),
    });
  }
  await writeJson(path.join(BASELINE, "inputs.json"), {
    run_inputs: entries,
    all_samples: sampleList,
  });
}

// ---------- run 実行 ----------

function runAl(args, { killWhen } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(cfg_binary(), args, {
      cwd: ROOT,
      stdio: ["ignore", "inherit", "inherit"],
    });
    const mon = monitorProcess(child, 1000);
    let killed = false;
    const poll = killWhen
      ? setInterval(async () => {
          try {
            if (await killWhen()) {
              killed = true;
              child.kill();
            }
          } catch {
            /* manifest 未出現等 */
          }
        }, 500)
      : null;
    child.on("exit", async (code) => {
      if (poll) clearInterval(poll);
      const stats = await mon.finish();
      resolve({ code, killed, stats });
    });
    child.on("error", reject);
  });
}

const cfg_binary = () => path.join(ROOT, "target", "release", "al");

async function jobDirOf(baseDir) {
  const ents = await readdir(baseDir, { withFileTypes: true });
  const jobs = ents.filter((e) => e.isDirectory() && e.name.startsWith("JOB-"));
  if (jobs.length !== 1) {
    throw new Error(`expected 1 job in ${baseDir}: ${jobs.map((j) => j.name)}`);
  }
  return path.join(baseDir, jobs[0].name);
}

async function executeRun(run) {
  const baseDir = path.join(TMP_RUNS, run.name);
  await rm(baseDir, { recursive: true, force: true });
  await mkdir(baseDir, { recursive: true });
  const inputArgs = run.inputs.map((i) => `${i.role}:${path.join(ROOT, i.path)}`).join(",");
  const idArgs = run.inputs.map((i) => i.id).join(",");
  const args = ["run", "--base-dir", baseDir, "--input", inputArgs, "--input-ids", idArgs];
  if (run.stages) args.push("--stages", run.stages);
  if (run.config) args.push("--config", path.join(ROOT, run.config));
  console.log(`[run ${run.name}] al ${args.join(" ")}`);

  const wall0 = performance.now();
  let stats;
  if (run.resumeAfterStage) {
    // 全 stage plan で run し、manifest ポーリングで指定 stage が
    // Completed になった境界で kill → al resume で完走
    const base = baseDir;
    const killWhen = async () => {
      const base2 = await jobDirOf(base).catch(() => null);
      if (!base2) return false;
      const mfile = path.join(base2, "manifest.json");
      if (!existsSync(mfile)) return false;
      const m = JSON.parse(await readFile(mfile, "utf8"));
      const st = m.stages?.find((s) => s.stage === run.resumeAfterStage);
      return st?.status === "completed";
    };
    const r1 = await runAl(args, { killWhen });
    const jobDir = await jobDirOf(baseDir);
    const jobId = path.basename(jobDir);
    console.log(`[run ${run.name}] killed after ${run.resumeAfterStage}; resuming ${jobId}`);
    const r2 = await runAl(["resume", "--base-dir", baseDir, "--job-id", jobId]);
    if (r2.code !== 0) throw new Error(`resume failed: code ${r2.code}`);
    stats = {
      first: r1.stats,
      second: r2.stats,
      killed_at_stage: run.resumeAfterStage,
    };
    stats.cpu_sec_last_sample =
      (r1.stats.cpu_sec_last_sample ?? 0) + (r2.stats.cpu_sec_last_sample ?? 0);
    stats.peak_rss_bytes = Math.max(r1.stats.peak_rss_bytes ?? 0, r2.stats.peak_rss_bytes ?? 0);
    stats.method = `${r1.stats.method} x2 (resume)`;
    stats.sample_count = r1.stats.sample_count + r2.stats.sample_count;
  } else {
    const r = await runAl(args);
    if (r.code !== 0) throw new Error(`run failed: code ${r.code}`);
    stats = r.stats;
  }
  const wallMs = performance.now() - wall0;
  const jobDir = await jobDirOf(baseDir);
  return { jobDir, wallMs, procStats: stats };
}

// ---------- 収集 ----------

function classifyIndex(rel, contracts) {
  for (const cls of ["bit-exact", "tolerance", "record-only"]) {
    for (const g of contracts.indexClasses?.[cls] ?? []) {
      if (globMatch(g, rel)) return cls;
    }
  }
  return contracts.defaultIndexClass ?? "record-only";
}

function nonfiniteTag(v) {
  if (typeof v === "number" && !Number.isFinite(v)) {
    return { $nonfinite: Number.isNaN(v) ? "NaN" : v > 0 ? "Infinity" : "-Infinity" };
  }
  return v;
}

function encodeValues(data) {
  return Array.from(data, nonfiniteTag);
}

function fingerprint(data) {
  let count = 0;
  let min = Infinity;
  let max = -Infinity;
  let sum = 0;
  const nonfinite = {};
  for (const v of data) {
    count++;
    if (!Number.isFinite(v)) {
      const k = Number.isNaN(v) ? "NaN" : v > 0 ? "Infinity" : "-Infinity";
      nonfinite[k] = (nonfinite[k] ?? 0) + 1;
      continue;
    }
    if (v < min) min = v;
    if (v > max) max = v;
    sum += v;
  }
  return {
    count,
    min: nonfiniteTag(min),
    max: nonfiniteTag(max),
    sum: nonfiniteTag(sum),
    nonfinite_counts: nonfinite,
  };
}

async function collectRun(run, jobDir, wallMs, procStats, contracts) {
  const outDir = path.join(RUNS_OUT, run.name);
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });
  const inputAbs = {};
  for (const i of run.inputs) {
    inputAbs[`<IN:${i.role}>`] = path.join(ROOT, i.path);
  }
  const absPaths = {
    "<WT>": ROOT,
    "<BASE>": jobDir,
    ...inputAbs,
  };
  const norm = await buildRunNormalizer(jobDir, { absPaths });
  const files = await walkFiles(jobDir);

  // 1) artifacts.index.json
  const index = {};
  for (const rel of files) {
    const abs = path.join(jobDir, rel);
    index[norm.normalizePath(rel)] = {
      size: await fileSize(abs),
      sha256: await sha256File(abs),
      class: classifyIndex(rel, contracts),
    };
  }
  await writeJson(path.join(outDir, "artifacts.index.json"), index);

  // 2) manifest コピー + 正規化
  const manifest = await readJson(path.join(jobDir, "manifest.json"));
  await cp(path.join(jobDir, "manifest.json"), path.join(outDir, "manifest.json"));
  await writeJson(path.join(outDir, "manifest.normalized.json"), norm.normalizeValue(manifest));

  // 3) golden: 小規模 JSON の正規化写し + table 行 + tensor payload
  const goldenDir = path.join(outDir, "golden");
  const goldenSet = contracts.goldenJsonGlobs ?? [];
  const skipSet = contracts.goldenJsonSkipGlobs ?? [];
  for (const rel of files) {
    if (!rel.endsWith(".json")) continue;
    if (skipSet.some((g) => globMatch(g, rel))) continue;
    if (!goldenSet.some((g) => globMatch(g, rel))) continue;
    const v = await readJson(path.join(jobDir, rel));
    const dst = path.join(goldenDir, norm.normalizePath(rel));
    await writeJson(dst, norm.normalizeValue(v));
  }
  // table 行デコード
  const { tables, tensors } = await findTablesAndTensors(jobDir, walkFiles);
  for (const tdir of tables) {
    const t = await readTable(path.join(jobDir, tdir));
    const dst = path.join(goldenDir, norm.normalizePath(`${tdir}.rows.json`));
    await writeJson(dst, norm.normalizeValue(tableToRows(t)));
  }
  // tensor payload(小: 全値 / 大: fingerprint)。record-only 面
  // (export・visualization)は payload dump 自体をスキップする
  const payloadSkip = contracts.payloadSkipGlobs ?? [];
  const payloadSkipped = (rel) => payloadSkip.some((g) => globMatch(g, rel));
  for (const adir of tensors) {
    if (payloadSkipped(`${adir}/x.bin`) || payloadSkipped(adir)) continue;
    const { meta, data } = await readTensor(path.join(jobDir, adir));
    const base = path.join(goldenDir, "payloads", norm.normalizePath(`${adir}.payload.json`));
    if (data.length <= SMALL_PAYLOAD_MAX) {
      await writeJson(base, {
        shape: meta.shape,
        values: encodeValues(data),
      });
    } else {
      await writeJson(base, {
        shape: meta.shape,
        fingerprint: fingerprint(data),
      });
    }
  }
  // tolerance class の .wav payload → fingerprint
  for (const rel of files) {
    if (!rel.endsWith(".wav")) continue;
    if (payloadSkipped(rel)) continue;
    if (classifyIndex(rel, contracts) !== "tolerance") continue;
    const { fmt, samples } = await readWav(path.join(jobDir, rel));
    const dst = path.join(goldenDir, "payloads", norm.normalizePath(`${rel}.payload.json`));
    await writeJson(dst, {
      format: fmt,
      fingerprint:
        samples.length <= SMALL_PAYLOAD_MAX
          ? { values: encodeValues(samples) }
          : fingerprint(samples),
    });
  }

  // 4) performance.json
  const perf = {
    e2e_wall_ms: Math.round(wallMs),
    stages: Object.fromEntries(
      (manifest.stages ?? []).map((s) => [
        s.stage,
        { status: s.status, duration_ms: s.duration_ms ?? null },
      ]),
    ),
    process: procStats,
    io_estimate: {
      input_bytes: (
        await Promise.all(run.inputs.map((i) => fileSize(path.join(ROOT, i.path))))
      ).reduce((a, b) => a + b, 0),
      output_bytes: (await Promise.all(files.map((r) => fileSize(path.join(jobDir, r))))).reduce(
        (a, b) => a + b,
        0,
      ),
      note: "近似値: 読み=入力総サイズ、書き=成果物総サイズ。stage 間の中間再読込は含まない",
    },
    not_measured: [
      "stage 別 busy/CPU/RSS/IO(外部ポーリングでは stage 境界に帰属不可)",
      "alloc(bench_alloc.rs 側で別途計測)",
    ],
    artifact_size_by_area: {},
  };
  const areaBytes = {};
  for (const rel of files) {
    const area = rel.split("/")[0];
    areaBytes[area] = (areaBytes[area] ?? 0) + (await fileSize(path.join(jobDir, rel)));
  }
  perf.artifact_size_by_area = areaBytes;
  await writeJson(path.join(outDir, "performance.json"), perf);

  // 5) inspect.json(record-only)
  const jobId = path.basename(jobDir);
  const inspect = trySh(path.join(ROOT, "target", "release", "al"), [
    "inspect",
    "--base-dir",
    path.dirname(jobDir),
    "--job-id",
    jobId,
  ]);
  if (inspect) await writeFile(path.join(outDir, "inspect.json"), inspect);
  return { outDir, mapping: norm.mapping };
}

// ---------- メイン ----------

async function main() {
  const args = process.argv.slice(2);
  const onlyRun = args.includes("--run") ? args[args.indexOf("--run") + 1] : null;
  const skipRuns = args.includes("--skip-runs");
  const collectOnly = args.includes("--collect-only")
    ? args[args.indexOf("--collect-only") + 1]
    : null;

  const cfg = await readJson(path.join(BASELINE, "config.json"));
  const contracts = await readJson(path.join(BASELINE, "contracts.json"));
  await mkdir(BASELINE, { recursive: true });

  await writeEnvironment(cfg);
  await ensureDerivedInputs(cfg);
  await writeInputs(cfg);

  for (const run of cfg.runs) {
    if (onlyRun && run.name !== onlyRun) continue;
    let jobDir;
    let wallMs = null;
    let procStats = { note: "collect-only: 実行相スキップ" };
    if (collectOnly) {
      if (!collectOnly.startsWith(`${run.name}:`)) continue;
      jobDir = path.resolve(ROOT, collectOnly.slice(run.name.length + 1));
    } else if (!skipRuns) {
      const r = await executeRun(run);
      jobDir = r.jobDir;
      wallMs = r.wallMs;
      procStats = r.procStats;
    } else {
      continue;
    }
    console.log(`[collect ${run.name}] ${jobDir}`);
    await collectRun(run, jobDir, wallMs ?? 0, procStats, contracts);
    // 生成物は回収後に削除(disk 圧迫回避)
    await rm(path.join(TMP_RUNS, run.name), { recursive: true, force: true });
  }
  console.log("[capture] done");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

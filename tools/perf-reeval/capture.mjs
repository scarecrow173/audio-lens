// tools/perf-reeval/capture.mjs
// perf Issue #76/#71/#54/#60 移行後再評価の計測ドライバ。
// baseline/config.json の run 定義を使って `al run` を JSON ログ付きで
// 実行し、manifest stage 時間・tracing span close(busy)・計時イベントを
// 集計して docs/impl/ir/perf-reeval-2026-10/<run>.json に保存する。
//
// 使い方:
//   node tools/perf-reeval/capture.mjs --run <name> [--keep-job]
// 前提: target/release/al ビルド済み(計時 instrumentation 入り可)、
//       samples/_runs/decoded/*.avi 派生済み。
import { spawn } from "node:child_process";
import { mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const OUT_BASE = path.join(ROOT, "docs", "impl", "ir", "perf-reeval-2026-10");
const RUNS_BASE = path.join(ROOT, "samples", "_runs", "perf-reeval");
const BINARY = path.join(ROOT, "target", "release", "al");
const LOG_DIRECTIVE = "al_locate=info,al_analysis=info,al_job=info";

// span close イベントの "time.busy" 文字列 ("1.5s" "139ms" "25.3µs" "2m") を ms に解釈する。
function parseDur(s) {
  if (typeof s !== "string") return null;
  const m = s.match(/^([0-9.]+)\s*(ns|µs|us|ms|s|m)$/);
  if (!m) return null;
  const v = parseFloat(m[1]);
  const unit = { ns: 1e-6, µs: 1e-3, us: 1e-3, ms: 1, s: 1e3, m: 6e4 }[m[2]];
  return v * unit;
}

async function jobDirOf(baseDir) {
  const ents = await readdir(baseDir, { withFileTypes: true });
  const jobs = ents.filter((e) => e.isDirectory() && e.name.startsWith("JOB-"));
  if (jobs.length !== 1)
    throw new Error(`expected 1 job in ${baseDir}: ${jobs.map((j) => j.name)}`);
  return path.join(baseDir, jobs[0].name);
}

async function parseLog(logFile) {
  const text = await readFile(logFile, "utf8");
  const spans = []; // {name, fields, busy_ms, ts}
  const events = []; // {message, fields, target, ts}
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    let rec;
    try {
      rec = JSON.parse(line);
    } catch {
      continue;
    }
    const f = rec.fields ?? {};
    if (f.message === "close" && rec.span) {
      spans.push({
        name: rec.span.name,
        fields: rec.span,
        busy_ms: parseDur(f["time.busy"]),
        ts: rec.timestamp,
      });
    } else if (f.message) {
      events.push({
        message: f.message,
        target: rec.target,
        fields: f,
        ts: rec.timestamp,
      });
    }
  }
  return { spans, events };
}

function aggregate(spans, events) {
  const bySpan = {};
  for (const s of spans) {
    const e = bySpan[s.name] ?? { calls: 0, busy_ms: 0, instances: [] };
    bySpan[s.name] = e;
    e.calls += 1;
    e.busy_ms += s.busy_ms ?? 0;
    e.instances.push({ ...s.fields, busy_ms: s.busy_ms });
  }
  const stageEvents = events
    .filter((e) => e.message === "stage completed")
    .map((e) => ({ stage: e.fields.stage, duration_ms: e.fields.duration_ms }));
  return {
    span_busy: Object.fromEntries(
      Object.entries(bySpan).map(([k, v]) => [
        k,
        { calls: v.calls, busy_ms: Math.round(v.busy_ms) },
      ]),
    ),
    span_instances: Object.fromEntries(Object.entries(bySpan).map(([k, v]) => [k, v.instances])),
    stage_events: stageEvents,
    window_anchors_in: events.filter((e) => e.message === "window_anchors_in").map((e) => e.fields),
    method_tasks_pool: events.filter((e) => e.message === "method_tasks_pool").map((e) => e.fields),
    ref_phase: events.filter((e) => e.message === "ref_phase").map((e) => e.fields),
    band_gain_phase: events.filter((e) => e.message === "band_gain_phase").map((e) => e.fields),
  };
}

async function main() {
  const args = process.argv.slice(2);
  const onlyRun = args.includes("--run") ? args[args.indexOf("--run") + 1] : null;
  const keepJob = args.includes("--keep-job");
  const skipRun = args.includes("--skip-run");
  if (!onlyRun) {
    console.error(
      "usage: node tools/perf-reeval/capture.mjs --run <name> [--keep-job] [--skip-run]",
    );
    process.exit(1);
  }
  const cfg = JSON.parse(await readFile(path.join(ROOT, "baseline", "config.json"), "utf8"));
  const run = cfg.runs.find((r) => r.name === onlyRun);
  if (!run) throw new Error(`run not in baseline/config.json: ${onlyRun}`);
  const stagesOverride = args.includes("--stages")
    ? args[args.indexOf("--stages") + 1]
    : run.stages;
  const outName = args.includes("--out") ? args[args.indexOf("--out") + 1] : run.name;

  const baseDir = path.join(RUNS_BASE, run.name);
  let jobDir = null;
  let e2eWallMs = null;
  if (!skipRun) {
    await rm(baseDir, { recursive: true, force: true });
    await mkdir(baseDir, { recursive: true });
    const inputArgs = run.inputs.map((i) => `${i.role}:${path.join(ROOT, i.path)}`).join(",");
    const idArgs = run.inputs.map((i) => i.id).join(",");
    const alArgs = [
      "run",
      "--base-dir",
      baseDir,
      "--input",
      inputArgs,
      "--input-ids",
      idArgs,
      "--log",
      LOG_DIRECTIVE,
      "--log-format",
      "file=json",
    ];
    if (stagesOverride) alArgs.push("--stages", stagesOverride);
    if (run.config) alArgs.push("--config", path.join(ROOT, run.config));
    console.log(`[run ${run.name}] al ${alArgs.join(" ")}`);
    const wall0 = performance.now();
    const code = await new Promise((resolve, reject) => {
      const child = spawn(BINARY, alArgs, { cwd: ROOT, stdio: ["ignore", "inherit", "inherit"] });
      child.on("exit", resolve);
      child.on("error", reject);
    });
    const wallMs = performance.now() - wall0;
    if (code !== 0) throw new Error(`al run failed: code ${code}`);
    jobDir = await jobDirOf(baseDir);
    console.log(`[run ${run.name}] e2e wall ${(wallMs / 1000).toFixed(1)}s`);
    e2eWallMs = Math.round(wallMs);
  } else {
    jobDir = await jobDirOf(baseDir);
  }

  const manifest = JSON.parse(await readFile(path.join(jobDir, "manifest.json"), "utf8"));
  const logDir = path.join(baseDir, "logs");
  const logFiles = (await readdir(logDir)).filter((f) => f.endsWith(".log"));
  const { spans, events } = await parseLog(path.join(logDir, logFiles[0]));
  const env = {
    recorded_at: new Date().toISOString(),
    commit: (await import("node:child_process"))
      .execFileSync("git", ["rev-parse", "HEAD"], { cwd: ROOT, encoding: "utf8" })
      .trim(),
    worktree_dirty: true, // 一時計時 instrumentation 込みの計測
    note: "instrumented build: window_anchors_in/method_tasks/prepare_*/band_gain_phase/ref_phase",
  };
  const out = {
    run: run.name,
    stages_filter: stagesOverride ?? null,
    job_id: path.basename(jobDir),
    e2e_wall_ms: e2eWallMs,
    environment: env,
    stages: Object.fromEntries(
      (manifest.stages ?? []).map((s) => [
        s.stage,
        { status: s.status, duration_ms: s.duration_ms ?? null },
      ]),
    ),
    ...aggregate(spans, events),
  };
  await mkdir(OUT_BASE, { recursive: true });
  const outPath = path.join(OUT_BASE, `${outName}.json`);
  await writeFile(outPath, JSON.stringify(out, null, 2));
  console.log(`[collect ${run.name}] -> ${outPath}`);
  if (!keepJob) await rm(baseDir, { recursive: true, force: true });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

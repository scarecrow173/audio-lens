// issue-104 性能比較: baseline/runs/<run>-golden98/performance.json(#98 凍結、
// Ruviform 基盤化前)と baseline/runs/<run>/performance.json(#104 変更後
// capture)を突合し、e2e wall・共通 stage 時間・CPU・peak RSS・生成物 bytes の
// 増減を列挙する。#106 で visualization stage が撤去されたため stage 比較は
// 両側に存在する共通 stage のみ対象とする。
// 使い方: node tools/baseline/lib/perf-compare-104.mjs [--json <out>]
import { existsSync, readFileSync, writeFileSync } from "node:fs";

const RUNS = "baseline/runs";
const RUN_LIST = [
  "canonical",
  "variant-desync",
  "variant-clipping",
  "variant-reverb",
  "variant-filter",
  "variant-clicks",
  "small",
  "resume",
  "aa",
];

const pct = (oldV, newV) => (oldV > 0 ? ((newV - oldV) / oldV) * 100 : null);
const fmtPct = (v) => (v == null ? "n/a" : `${v >= 0 ? "+" : ""}${v.toFixed(1)}%`);

const report = {};
for (const run of RUN_LIST) {
  const oldF = `${RUNS}/${run}-golden98/performance.json`;
  const newF = `${RUNS}/${run}/performance.json`;
  if (!existsSync(oldF) || !existsSync(newF)) continue;
  const o = JSON.parse(readFileSync(oldF, "utf8"));
  const n = JSON.parse(readFileSync(newF, "utf8"));
  const stages = {};
  for (const s of new Set([...Object.keys(o.stages ?? {}), ...Object.keys(n.stages ?? {})])) {
    const ov = o.stages?.[s]?.duration_ms;
    const nv = n.stages?.[s]?.duration_ms;
    stages[s] = {
      old_ms: ov ?? null,
      new_ms: nv ?? null,
      pct: ov != null && nv != null ? pct(ov, nv) : null,
      note: ov == null ? "added" : nv == null ? "removed" : undefined,
    };
  }
  // 共通 stage のみの合計(e2e の擬似正規化。#106 で visualization が
  // 除去されたため、stage 合計は共通 stage に限定して算出する)
  const commonStages = Object.keys(stages).filter(
    (s) => stages[s].old_ms != null && stages[s].new_ms != null,
  );
  const oldStageSum = commonStages.reduce((a, s) => a + stages[s].old_ms, 0);
  const newStageSum = commonStages.reduce((a, s) => a + stages[s].new_ms, 0);
  report[run] = {
    e2e_wall_ms: {
      old: o.e2e_wall_ms,
      new: n.e2e_wall_ms,
      pct: pct(o.e2e_wall_ms, n.e2e_wall_ms),
    },
    stage_sum_common_ms: {
      old: oldStageSum,
      new: newStageSum,
      pct: pct(oldStageSum, newStageSum),
      stages: commonStages,
    },
    cpu_sec_last_sample: {
      old: o.process?.cpu_sec_last_sample ?? null,
      new: n.process?.cpu_sec_last_sample ?? null,
      pct:
        o.process?.cpu_sec_last_sample && n.process?.cpu_sec_last_sample
          ? pct(o.process.cpu_sec_last_sample, n.process.cpu_sec_last_sample)
          : null,
    },
    peak_rss_bytes: {
      old: o.process?.peak_rss_bytes ?? null,
      new: n.process?.peak_rss_bytes ?? null,
      pct:
        o.process?.peak_rss_bytes && n.process?.peak_rss_bytes
          ? pct(o.process.peak_rss_bytes, n.process.peak_rss_bytes)
          : null,
    },
    output_bytes: {
      old: o.io_estimate?.output_bytes ?? null,
      new: n.io_estimate?.output_bytes ?? null,
      pct:
        o.io_estimate?.output_bytes && n.io_estimate?.output_bytes
          ? pct(o.io_estimate.output_bytes, n.io_estimate.output_bytes)
          : null,
    },
    stages,
  };
}

console.log("| run | e2e old ms | e2e new ms | Δ% | stageΣΔ% | CPUΔ% | RSSΔ% |");
console.log("| --- | --- | --- | --- | --- | --- | --- |");
for (const [run, r] of Object.entries(report)) {
  console.log(
    `| ${run} | ${r.e2e_wall_ms.old} | ${r.e2e_wall_ms.new} | ` +
      `${fmtPct(r.e2e_wall_ms.pct)} | ${fmtPct(r.stage_sum_common_ms.pct)} | ` +
      `${fmtPct(r.cpu_sec_last_sample.pct)} | ${fmtPct(r.peak_rss_bytes.pct)} |`,
  );
}
console.log("\nstage 詳細(共通 stage のみ、Δ% |new-old|>5% または removed/added を表示):");
for (const [run, r] of Object.entries(report)) {
  for (const [s, st] of Object.entries(r.stages)) {
    const flag =
      st.note || st.pct == null || Math.abs(st.pct) > 5 || Math.abs(st.new_ms - st.old_ms) > 500;
    if (flag) {
      console.log(
        `  ${run}/${s}: ${st.old_ms ?? "-"} -> ${st.new_ms ?? "-"} ms ` +
          `(${st.note ?? fmtPct(st.pct)})`,
      );
    }
  }
}

const ji = process.argv.indexOf("--json");
if (ji >= 0) {
  writeFileSync(process.argv[ji + 1], JSON.stringify(report, null, 2));
  console.log(`\nwrote ${process.argv[ji + 1]}`);
}

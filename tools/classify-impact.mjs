// Classify a pull_request file diff into AudioLens change classes and
// derive the required CI lane set (verify / ui-verify / webview-verify,
// each lane = 3 OS legs). Implements the AudioLens side of the generic
// Impact contract (ruviform #333; AudioLens #149):
//   - pull_request only. push:main / workflow_dispatch always run Full CI.
//   - unclassifiable or shared-infrastructure changes fall back to Full CI.
//   - a lane is `required` or `not_applicable`; NotApplicable is emitted
//     only with a recorded justification. `summary` re-checks these
//     outputs fail-closed — skipped/NotRun/Unsupported never count as Pass.
//   - selective skip is armed by repo variable CI_IMPACT_SELECTIVE=1
//     (Shadow Mode default: classification is recorded but every lane
//     still runs until shadow evidence shows no false negatives).
//
// The rule table below is the single source of truth for class → lane
// mapping; docs/designs/impact-based-ci-issue-149.md mirrors it for review.
//
// Usage:
//   node tools/classify-impact.mjs --event pull_request --files changed.txt
//   gh api repos/:owner/:repo/pulls/N/files --jq '.[].filename' \
//     | node tools/classify-impact.mjs --event pull_request --files - \
//         --selective "${CI_IMPACT_SELECTIVE:-0}"
//   node tools/classify-impact.mjs --event pull_request --fallback
//
// Emits `mode`, `classes`, `<lane>` (effective), `<lane>_class` (raw
// classification) and `<lane>_reason` to GITHUB_OUTPUT, plus a
// classification evidence table to GITHUB_STEP_SUMMARY. Exit 0 on
// success — classification failures degrade to Full CI (safe side),
// never to a red gate.

import { appendFileSync, readFileSync } from "node:fs";

// Lane output names (GITHUB_OUTPUT-safe). Display name = name.replace("_", "-").
const LANES = ["verify", "ui_verify", "webview_verify"];

const V = { verify: true };
const U = { ui_verify: true };
const W = { webview_verify: true };
const merge = (...xs) => Object.assign({}, ...xs);
const ALL = merge(V, U, W);
const VW = merge(V, W);

const WHY_NA = "変更分類は当該 lane の検証対象外";

// Ordered rules — first match wins. `lanes` = lane set the class must run.
const RULES = [
  {
    cls: "ci",
    lanes: ALL,
    re: /^\.github\//,
    why: "workflow/CI 定義の変更 — 変更内容自身を全 lane で再検証",
  },
  {
    cls: "ci",
    lanes: ALL,
    re: /^\.pre-commit-config\.yaml$/,
    why: "prek hygiene 定義の変更 — 全 lane 必須",
  },
  {
    cls: "cargo-lock",
    lanes: ALL,
    re: /(^|\/)Cargo\.(toml|lock)$/,
    why: "workspace manifest/lock(ruviform resolved SHA 含む)— Consumer 回帰契約で全 lane 必須",
  },
  {
    cls: "toolchain",
    lanes: ALL,
    re: /^mise\.(toml|lock)$|^\.cargo\//,
    why: "共有 toolchain/ cargo 設定 — Full CI(安全側)",
  },
  {
    cls: "ui-vendor",
    lanes: U,
    re: /^tools\/sync-ruviform-ui\.mjs$/,
    why: "ui vendor 同期ツール — ui-verify のみが消費",
  },
  {
    cls: "docs",
    lanes: {},
    re: /(^|\/)docs\//,
    why: "文書 — 実行成果物への影響なし(prek hygiene が検査)",
  },
  {
    cls: "docs",
    lanes: {},
    re: /\.md$|^LICENSE|^NOTICE/,
    why: "文書 — 実行成果物への影響なし(prek hygiene が検査)",
  },
  {
    cls: "ci-scripts",
    lanes: ALL,
    re: /^tools\//,
    why: "CI/検証スクリプト — 安全側 Full CI",
  },
  {
    cls: "ts",
    lanes: U,
    re: /^ui\//,
    why: "ui/ headless TS 層 — cargo workspace 外、ui-verify のみ",
  },
  {
    cls: "lfs-golden",
    lanes: V,
    re: /^\.gitattributes$/,
    why: "LFS filter 定義 — fixture 復元を verify で再検証",
  },
  {
    cls: "lfs-golden",
    lanes: V,
    re: /^(assets|samples|dictionaries|baseline)\//,
    why: "Golden/LFS fixture — workspace test が消費(al-gui webview lane は参照しない)",
  },
  {
    cls: "cli",
    lanes: V,
    re: /^crates\/al-cli\//,
    why: "al-cli(Export 含む)— al-gui 依存 cone 外、verify のみ",
  },
  {
    cls: "observability",
    lanes: V,
    re: /^crates\/al-observability\//,
    why: "al-observability — al-cli 経由のみ、al-gui cone 外",
  },
  {
    cls: "gui",
    lanes: VW,
    re: /^crates\/al-gui\//,
    why: "al-gui(headless + webview feature — feature 内分離は path 分解不能)",
  },
  { cls: "rust-core", lanes: VW, re: /^crates\/al-core\//, why: "al-core — 全 crate の基盤" },
  {
    cls: "service",
    lanes: VW,
    re: /^crates\/al-(job|service)\//,
    why: "al-job/al-service — al-gui 依存 cone 内",
  },
  {
    cls: "analysis",
    lanes: VW,
    re: /^crates\/al-(analysis|locate)\//,
    why: "al-analysis/al-locate — al-gui 依存 cone 内",
  },
  {
    cls: "source",
    lanes: VW,
    re: /^crates\/al-media\//,
    why: "al-media(source ingest)— al-gui 依存 cone 内",
  },
  {
    cls: "dataset",
    lanes: VW,
    re: /^crates\/al-dataset\//,
    why: "al-dataset — al-gui 依存 cone 内",
  },
  {
    cls: "testkit",
    lanes: VW,
    re: /^crates\/al-testkit\//,
    why: "al-testkit — dev-dep で al-gui test cone 内",
  },
  {
    cls: "unclassified",
    lanes: ALL,
    re: /^crates\//,
    why: "未知の crate パス — 安全側 Full CI",
  },
  {
    cls: "lfs-golden",
    lanes: V,
    re: /\.(png|jpe?g|gif|pdf|wav|flac|mp3|ogg|opus|aac|m4a|aiff?|mp4|mov|mkv|avi|webm|m2ts|glb|bin|parquet|arrow|npy|npz|zip)$/i,
    why: "LFS fixture 拡張子 — workspace test が消費",
  },
  {
    cls: "docs-config",
    lanes: {},
    re: /^\.(agents|claude|codex|cursor|windsurf|serena|sdp|vscode)\//,
    why: "agent 設定 — 実行成果物への影響なし(prek hygiene が検査)",
  },
  {
    cls: "docs-config",
    lanes: {},
    re: /^\.(mcp\.json|gitignore|editorconfig|worktreeinclude)$|^biome\.jsonc$|^apm\.(yml|lock\.yaml)$|^AGENTS\.md$/,
    why: "repo/agent 設定 — 実行成果物への影響なし(prek hygiene が検査)",
  },
];

function argValue(name) {
  const i = process.argv.indexOf(`--${name}`);
  return i === -1 ? null : process.argv[i + 1];
}
const hasFlag = (name) => process.argv.includes(`--${name}`);

const eventName = argValue("event") || process.env.GITHUB_EVENT_NAME || "unknown";
const selective = argValue("selective") === "1";
const fallback = hasFlag("fallback");

let files = [];
if (!fallback && eventName === "pull_request") {
  const src = argValue("files");
  const raw = src === "-" ? readFileSync(0, "utf8") : src ? readFileSync(src, "utf8") : "";
  files = raw
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
}

/** Map every changed file through the rule table (first match wins). */
function classify() {
  const hits = new Map(); // cls -> { files, lanes, why }
  const add = (cls, lanes, why, file) => {
    if (!hits.has(cls)) {
      hits.set(cls, { files: [], lanes, why });
    }
    hits.get(cls).files.push(file);
  };
  for (const f of files) {
    const rule = RULES.find((r) => r.re.test(f));
    if (rule === undefined) {
      add("unclassified", ALL, "分類不能 — 安全側 Full CI", f);
    } else {
      add(rule.cls, rule.lanes, rule.why, f);
    }
  }
  return hits;
}

const hits = eventName === "pull_request" && !fallback ? classify() : new Map();
const mode = eventName !== "pull_request" || fallback ? "full" : selective ? "selective" : "shadow";
const modeReason =
  eventName !== "pull_request"
    ? `event=${eventName} — pull_request 以外は Full CI 契約`
    : fallback
      ? "変更ファイル取得失敗 — 安全側 Full CI"
      : selective
        ? "selective armed (CI_IMPACT_SELECTIVE=1)"
        : "Shadow Mode — 分類のみ記録、全 lane 実行(CI_IMPACT_SELECTIVE 未設定)";

const classified = {};
const reasons = {};
for (const lane of LANES) {
  const reqClasses = [...hits.entries()].filter(([, h]) => h.lanes[lane]).map(([c]) => c);
  classified[lane] =
    mode === "full" ? "required" : reqClasses.length > 0 ? "required" : "not_applicable";
  reasons[lane] =
    mode === "full"
      ? modeReason
      : reqClasses.length > 0
        ? `required by ${reqClasses.join(", ")}`
        : WHY_NA;
}
const effective = {};
for (const lane of LANES) {
  effective[lane] = mode === "selective" ? classified[lane] : "required";
}

// Evidence table → step summary (auditable on the PR Checks page).
const disp = (lane) => lane.replaceAll("_", "-");
{
  const lines = [];
  lines.push(`## impact — change classification (mode: ${mode})`);
  lines.push("");
  lines.push(`- event: ${eventName} · files: ${files.length} · ${modeReason}`);
  if (hits.has("unclassified")) {
    lines.push(
      `- unclassified files: ${hits
        .get("unclassified")
        .files.map((f) => `\`${f}\``)
        .join(", ")}`,
    );
  }
  lines.push("");
  lines.push("| class | files | lanes | 根拠 |");
  lines.push("|---|---|---|---|");
  for (const [cls, h] of hits) {
    const laneList =
      LANES.filter((l) => h.lanes[l])
        .map(disp)
        .join(", ") || "(hygiene のみ)";
    lines.push(`| ${cls} | ${h.files.length} | ${laneList} | ${h.why} |`);
  }
  if (hits.size === 0) {
    lines.push("| (変更なし) | 0 | (hygiene のみ) | 空の diff |");
  }
  lines.push("");
  lines.push("| lane | classified | effective | reason |");
  lines.push("|---|---|---|---|");
  for (const lane of LANES) {
    lines.push(
      `| ${disp(lane)} (3 OS) | ${classified[lane]} | ${effective[lane]} | ${reasons[lane]} |`,
    );
  }
  const table = `${lines.join("\n")}\n`;
  process.stdout.write(table);
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, table);
  }
}

if (process.env.GITHUB_OUTPUT) {
  const out = [
    `mode=${mode}`,
    `classes=${[...hits.keys()].join(",") || "(none)"}`,
    ...LANES.map((l) => `${l}=${effective[l]}`),
    ...LANES.map((l) => `${l}_class=${classified[l]}`),
    ...LANES.map((l) => `${l}_reason=${reasons[l]}`),
  ];
  appendFileSync(process.env.GITHUB_OUTPUT, `${out.join("\n")}\n`);
}

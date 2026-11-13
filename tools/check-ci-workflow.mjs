// Issue #158 (安全な第一段階) ci.yml の Full CI 構造契約を fail-closed
// で検査する。actionlint の文法検査では担保できない「job 集合・
// needs・集約分離・選択実行機構の不在」を、YAML の indentation
// 構造に基づいて静的に照合する(ruviform-lock-rev.mjs と同じく
// 依存ゼロ・構造検査・任意の文字列 grep ではない)。
//
// 契約(正本は docs/designs/full-ci-ci-summary-issue-158.md +
// docs/designs/ci-dispatch-only-issue-147.md +
// docs/designs/ci-os-l1-gpui-desktop.md(Issue #206 で gpui lane
// 追加・job 集合 14 へ拡張)):
// - trigger: workflow_dispatch のみ(Issue #147/#158 最終段階で
//   pull_request/push を撤去。schedule/workflow_run 等他 trigger も
//   持たない)。dispatch は inputs.expected_sha を required で持つ。
// - job 集合は固定: expected-sha, ruviform-access, hygiene,
//   verify, verify-macos, ui-verify, ui-verify-macos,
//   webview-verify, webview-verify-macos, gpui-verify,
//   gpui-verify-macos, gui-real,
//   ci-summary, summary(追加・削除・rename は契約違反)。
// - lane jobs は needs: [ruviform-access, expected-sha] で
//   両 preflight の success を `if:` に課す(常時実行)。
// - expected-sha job は全 run で expected_sha を無条件照合する
//   (github.event_name を参照する if: を持たない)。
// - ci-summary は全 job を needs して `if: always()` —
//   Full CI verdict。macOS legs を含む。
// - summary は macOS legs を含まない needs + `if: always()` —
//   Ruleset Required context `summary` の互換 gate(#157 の
//   macOS 非含有 scope 維持。gpui-verify は非 macOS lane と
//   して含み、gpui-verify-macos は含まない)。
// - 選択実行機構の残滓(impact job/classify-impact/
//   CI_IMPACT_SELECTIVE/期待集合 not_applicable/macos-summary)は
//   存在してはならない。
// - workflow_dispatch inputs(未検証ユーザー入力)は run 本文へ式
//   展開しない(always() な集約 job は入力拒否後も実行されるため、
//   env/if など値としてのみ評価される文脈で扱う)。
//
// Usage:
//   node tools/check-ci-workflow.mjs            ci.yml を検査
//   node tools/check-ci-workflow.mjs <path>     任意ファイルを検査(負例 fixture 用)
// 成功時 exit 0 / 契約違反は全件列挙して exit 1。

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const WORKFLOW = resolve(".github/workflows/ci.yml");
const path = resolve(process.argv[2] ?? WORKFLOW);
const text = readFileSync(path, "utf8");
const lines = text.split("\n");

const failures = [];
function check(cond, msg) {
  if (!cond) failures.push(msg);
}

// --- indentation-aware structural helpers -------------------------------

// 0 インデントの top-level key ブロックを行配列で返す
// (例: topBlock("on") → `on:` 行の次の 0 indent までの行)。
function topBlock(key) {
  const start = lines.findIndex((l) => l === `${key}:`);
  if (start === -1) return null;
  const out = [];
  for (let i = start + 1; i < lines.length; i++) {
    const l = lines[i];
    if (/^\S/.test(l)) break; // 次の top-level key
    out.push(l);
  }
  return out;
}

// `jobs:` 配下の job id(2 空白 indent)ごとの行ブロックを Map で返す。
function jobBlocks() {
  const jobs = topBlock("jobs");
  if (!jobs) return null;
  const map = new Map();
  let cur = null;
  for (const l of jobs) {
    const m = l.match(/^ {2}([A-Za-z0-9_-]+):\s*$/);
    if (m) {
      cur = m[1];
      map.set(cur, []);
    } else if (cur && /^ {2}\S/.test(l)) {
      cur = null; // 同階層の別 key(jobs ブロック末端の防御)
    } else if (cur) {
      map.get(cur).push(l);
    }
  }
  return map;
}

// job ブロック内の `needs: [a, b]`(flow list 書法)を配列で返す。
function jobNeeds(block) {
  const m = block.join("\n").match(/^\s*needs:\s*\[([^\]]*)\]\s*$/m);
  if (!m) return null;
  return m[1]
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

const blockText = (block) => block.join("\n");

// --- trigger / env 契約 ---------------------------------------------------

const on = topBlock("on");
check(!!on, "`on:` ブロックが存在しない");
if (on) {
  const onText = on.join("\n");
  // trigger key は on: 直下の 2-indent key(inputs 等の nested key は
  // より深い indent のため拾われない)。
  const triggerKeys = on
    .map((l) => l.match(/^ {2}([A-Za-z_][A-Za-z0-9_]*):/))
    .filter(Boolean)
    .map((m) => m[1]);
  check(
    triggerKeys.length === 1 && triggerKeys[0] === "workflow_dispatch",
    `trigger が workflow_dispatch 唯一でない(actual: ${triggerKeys.join(", ")})`,
  );
  for (const bad of [
    "pull_request",
    "pull_request_target",
    "push",
    "schedule",
    "workflow_run",
    "issue_comment",
    "merge_group",
    "workflow_call",
  ]) {
    check(!triggerKeys.includes(bad), `禁止 trigger ${bad} が on: に存在する`);
  }
  check(
    /workflow_dispatch:\s*\n(?:\s+.*\n)*?\s+inputs:\s*\n(?:\s+.*\n)*?\s+expected_sha:\s*\n(?:\s+.*\n)*?\s+required:\s*true/.test(
      onText,
    ),
    "workflow_dispatch.inputs.expected_sha(required: true) が宣言されていない",
  );
}

const env = topBlock("env");
check(!!env, "`env:` ブロックが存在しない");
if (env) {
  check(
    !env.join("\n").includes("CI_IMPACT_SELECTIVE"),
    "env に CI_IMPACT_SELECTIVE が残っている(選択実行機構)",
  );
}

// --- 選択実行機構の残滓が全体に存在しないこと -------------------------------
// コメント内の言及(設計経緯の記録)は機構ではないため、各行から
// YAML コメント部を除いた実装部のみを検査対象にする。

const codeText = lines.map((l) => (/^\s*#/.test(l) ? "" : l.replace(/\s#.*$/, ""))).join("\n");

for (const token of [
  "classify-impact",
  "CI_IMPACT_SELECTIVE",
  "macos-summary",
  "not_applicable",
  "needs.impact",
  "steps.classify",
  "SHADOW",
]) {
  check(!codeText.includes(token), `workflow 内に選択実行機構の残滓 '${token}' が存在する`);
}

// --- 未検証入力の run 本文への式展開禁止 -----------------------------------
// ci-summary は `if: always()` のため expected-sha job が dispatch
// input を拒否しても実行される。github.event.inputs.* を run 本文へ
// 式展開すると、引用符等を含む入力が shell コードとして解釈され得る
// (env 値として渡せば shell にはデータとしてのみ到達する)。
// run: のブロック/インライン本文だけを抽出して検査する — env: 値や
// if: 式への入力式の使用は許可される境界であり違反ではない。
// ブロック内の行末 `#` 以降も GitHub 式展開は先に走るため、run 本文
// では行内コメントを除かない(全文を対象にする)。

const noFullCommentLines = lines.map((l) => (/^\s*#/.test(l) ? "" : l));

function runBodies() {
  const out = [];
  for (let i = 0; i < noFullCommentLines.length; i++) {
    const m = noFullCommentLines[i].match(/^(\s*)run:\s*(.*)$/);
    if (!m) continue;
    const indent = m[1].length;
    const inline = m[2].trim();
    if (inline.startsWith("|") || inline.startsWith(">")) {
      const body = [];
      for (let j = i + 1; j < noFullCommentLines.length; j++) {
        const l = noFullCommentLines[j];
        if (l.trim() && /^\s*/.exec(l)[0].length <= indent) break;
        body.push(l);
      }
      out.push(body.join("\n"));
    } else {
      out.push(inline);
    }
  }
  return out;
}

for (const body of runBodies()) {
  check(
    !/github\.event\.inputs\./.test(body),
    "run 本文へ github.event.inputs.* を式展開している(未検証入力が shell コードとして解釈され得る。env 経由で値として渡すこと)",
  );
}

// --- job 集合契約 ----------------------------------------------------------

const jobs = jobBlocks();
check(!!jobs, "`jobs:` ブロックが解析できない");

const EXPECTED_JOBS = [
  "expected-sha",
  "ruviform-access",
  "hygiene",
  "verify",
  "verify-macos",
  "ui-verify",
  "ui-verify-macos",
  "webview-verify",
  "webview-verify-macos",
  "gpui-verify",
  "gpui-verify-macos",
  "gui-real",
  "ci-summary",
  "summary",
];

if (jobs) {
  const actual = [...jobs.keys()];
  check(
    actual.length === EXPECTED_JOBS.length && EXPECTED_JOBS.every((j) => actual.includes(j)),
    `job 集合が契約と一致しない(actual: ${actual.join(", ")})`,
  );

  // lane jobs: needs [ruviform-access, expected-sha] + success if:
  const LANES = [
    "verify",
    "verify-macos",
    "ui-verify",
    "ui-verify-macos",
    "webview-verify",
    "webview-verify-macos",
    "gpui-verify",
    "gpui-verify-macos",
  ];
  for (const lane of LANES) {
    const block = jobs.get(lane);
    if (!block) continue;
    const needs = jobNeeds(block);
    check(
      !!needs &&
        needs.length === 2 &&
        needs.includes("ruviform-access") &&
        needs.includes("expected-sha"),
      `job ${lane}: needs が [ruviform-access, expected-sha] でない(actual: ${needs ? needs.join(",") : "none"})`,
    );
    const ifLine = blockText(block).match(/^\s*if:\s*(.+)$/m);
    check(
      !!ifLine &&
        ifLine[1].includes("needs.ruviform-access.result == 'success'") &&
        ifLine[1].includes("needs.expected-sha.result == 'success'"),
      `job ${lane}: if: が両 preflight の success を要求していない`,
    );
  }

  // expected-sha: dispatch 照合 step の存在 + 無条件実行(dispatch
  // 専用下で event 分岐を持つと照合が no-op 化し得るため禁止)
  const es = jobs.get("expected-sha");
  if (es) {
    const b = blockText(es);
    check(
      b.includes("git rev-parse HEAD") && b.includes("expected_sha"),
      "job expected-sha: checkout revision と expected_sha の照合 step が無い",
    );
    check(
      !b.includes("github.event_name"),
      "job expected-sha: github.event_name を参照している(dispatch 専用下では全 run が無条件照合する)",
    );
  }

  // hygiene: expected-sha を needs(prek --all-files は既存維持)
  const hy = jobs.get("hygiene");
  if (hy) {
    const needs = jobNeeds(hy);
    check(
      !!needs && needs.length === 1 && needs[0] === "expected-sha",
      `job hygiene: needs が [expected-sha] でない(actual: ${needs ? needs.join(",") : "none"})`,
    );
    check(
      blockText(hy).includes("prek run --all-files"),
      "job hygiene: prek run --all-files step が無い",
    );
  }

  // gui-real: not_run 証跡の維持
  const gr = jobs.get("gui-real");
  if (gr) {
    check(
      blockText(gr).includes("status=not_run") || blockText(gr).includes("not run"),
      "job gui-real: not_run 証跡出力が無い",
    );
  }

  // ci-summary: 全 job needs + always() + check 名
  const CISUMMARY_NEEDS = [
    "expected-sha",
    "ruviform-access",
    "hygiene",
    "verify",
    "verify-macos",
    "ui-verify",
    "ui-verify-macos",
    "webview-verify",
    "webview-verify-macos",
    "gpui-verify",
    "gpui-verify-macos",
    "gui-real",
  ];
  const cs = jobs.get("ci-summary");
  if (cs) {
    const needs = jobNeeds(cs);
    check(
      !!needs &&
        needs.length === CISUMMARY_NEEDS.length &&
        CISUMMARY_NEEDS.every((n) => needs.includes(n)),
      `job ci-summary: needs が全必要 job を含まない(actual: ${needs ? needs.join(",") : "none"})`,
    );
    check(
      /^\s*if:\s*always\(\)\s*$/m.test(blockText(cs)),
      "job ci-summary: `if: always()` が無い(完走しないと fail-closed にならない)",
    );
    check(
      /^\s*name:\s*ci-summary\s*$/m.test(blockText(cs)),
      "job ci-summary: `name: ci-summary` が無い(check context 契約)",
    );
  }

  // summary: macOS 非含有 needs + always() + check 名 `summary`
  const SUMMARY_NEEDS = [
    "expected-sha",
    "ruviform-access",
    "hygiene",
    "verify",
    "ui-verify",
    "webview-verify",
    "gpui-verify",
    "gui-real",
  ];
  const su = jobs.get("summary");
  if (su) {
    const needs = jobNeeds(su);
    check(
      !!needs &&
        needs.length === SUMMARY_NEEDS.length &&
        SUMMARY_NEEDS.every((n) => needs.includes(n)),
      `job summary: needs が Required context scope と一致しない(actual: ${needs ? needs.join(",") : "none"})`,
    );
    check(
      !needs || !needs.some((n) => n.endsWith("-macos")),
      "job summary: needs に macOS leg が含まれている(#157 の Required 非含有 scope を破壊)",
    );
    check(/^\s*if:\s*always\(\)\s*$/m.test(blockText(su)), "job summary: `if: always()` が無い");
    check(
      /^\s*name:\s*summary\s*$/m.test(blockText(su)),
      "job summary: `name: summary` が無い(Required context が死ぬ)",
    );
  }
}

// --- CI/Composite Action の実行OS分担契約 ----------------------------
// 本番CIファイルの検査時のみ、参照先Composite ActionのLint/Native境界も照合。
// fixture pathを明示した負例テストは従来どおりci.yml単独を検査する。
if (path === WORKFLOW) {
  const actionStep = (src, label) => {
    const start = src.indexOf("    - name: " + label + "\n");
    check(start >= 0, "composite step missing: " + label);
    if (start < 0) return "";
    const next = src.indexOf("\n    - name:", start + 1);
    return src.slice(start, next < 0 ? src.length : next);
  };

  const specs = [
    [".github/actions/verify/action.yml", "Lint workspace (Linux; mise run lint-full)"],
    [
      ".github/actions/webview-verify/action.yml",
      "Lint UI webview feature (Linux-only; mise run lint-ui-webview)",
    ],
    [
      ".github/actions/gpui-verify/action.yml",
      "Lint GPUI feature (Linux-only; mise run lint-ui-gpui)",
    ],
  ];

  for (const [file, lintLabel] of specs) {
    const body = readFileSync(resolve(file), "utf8");
    const lint = actionStep(body, lintLabel);
    check(
      lint.includes("if: inputs.os == 'ubuntu-latest'"),
      file + ": Clippy must run only in the canonical Linux lane",
    );

    if (file.includes("/verify/")) {
      const fmt = actionStep(body, "Check formatting (Linux canonical rustfmt)");
      check(fmt.includes("if: inputs.os == 'ubuntu-latest'"), "verify: rustfmt must be Linux-only");
      for (const [label, task] of [
        ["Type-check workspace (non-Linux; mise run check-full)", "check-full"],
        ["Build/link workspace (non-Linux; mise run build-full)", "build-full"],
      ]) {
        const step = actionStep(body, label);
        check(
          step.includes("if: inputs.os != 'ubuntu-latest'"),
          "verify: native " + task + " missing outside Linux",
        );
        check(step.includes("run: mise run " + task), "verify: wrong " + task + " command");
      }
      check(body.includes("run: mise run test-full"), "verify: native test-full was removed");
      check(
        body.includes("cargo build -p al-cli --profile ci"),
        "verify: headless CLI Build/Link missing",
      );
    } else {
      const suffix = file.includes("webview") ? "webview" : "gpui";
      check(
        body.includes("run: mise run check-ui-" + suffix),
        file + ": OS-specific type-check removed",
      );
      check(
        body.includes("run: mise run test-ui-" + suffix),
        file + ": OS-specific test removed",
      );
      if (suffix === "gpui") {
        check(
          body.includes("run: mise run build-ui-gpui"),
          file + ": native al-desktop Build/Link removed",
        );
      }
    }
  }
}

// --- 結果 -----------------------------------------------------------------

if (failures.length) {
  console.error(`check-ci-workflow: ${failures.length} 件の契約違反 (${path})`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log(`check-ci-workflow: Full CI 構造契約 OK (${path})`);

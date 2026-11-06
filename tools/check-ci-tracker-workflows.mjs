// Issue #159 (reconcile 展開) tracker 系 workflow 群の構造契約を
// fail-closed で検査する。actionlint の文法検査では担保できない
// 「共有 pin 一致・reconcile 入口の trigger/guard/権限/concurrency/
// mode・event 入口の trigger 不変」を、check-ci-workflow.mjs と同じく
// 依存ゼロの YAML indentation 構造検査で照合する。
//
// 契約(正本は docs/test-specs/ci-tracker-reconcile-issue-159.md):
// - 共有 pin: 3 ファイルの `repository: scarecrow173/ruviform`
//   checkout `ref:` が SHARED_CI_REF(正本)と一致する。
// - reconcile 入口: trigger は schedule + workflow_dispatch のみ
//   (workflow_run/pull_request/push 不在)。cron は分 != 0。
// - reconcile job: `if:` は schedule 無条件 + dispatch ref guard、
//   concurrency は event 入口と同一 group、permissions は
//   contents:read/actions:read/issues:write のみ、
//   timeout-minutes: 15。
// - checkout: 自 repo は default branch 式 pin、共有実装は
//   RUVIFORM_DEPLOY_KEY + persist-credentials: false。
// - mode 分離: 実行は `tracker.mjs --reconcile` 明示、非コメント行に
//   `TRACKER_EVENT_PATH:` env key を持たない。self-test が実行より前。
// - event 入口 `ci-issue-tracker.yml`: trigger は workflow_run のみ
//   (schedule/workflow_dispatch/pull_request/push 不在 — cron を
//   event 入口へ足す回帰を防ぐ)。実行は event モードのまま
//   (TRACKER_EVENT_PATH があり --reconcile を付けない)。
// - daily-ci.yml: 共有 pin + schedule/workflow_dispatch trigger の維持。
//
// Usage:
//   node tools/check-ci-tracker-workflows.mjs           repo root を検査
//   node tools/check-ci-tracker-workflows.mjs <root>    任意 root を検査(負例 fixture 用)
// 成功時 exit 0 / 契約違反は全件列挙して exit 1。

import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

// 共有実装の正本 pin。更新は全 workflow + この定数を同時に行う。
const SHARED_CI_REF = "eed590e0a3837e61c943d9de1d3efcfd7df32cb0";

const ROOT = resolve(process.argv[2] ?? ".");
const WF = join(ROOT, ".github", "workflows");
const RECONCILE = join(WF, "ci-issue-tracker-reconcile.yml");
const EVENT = join(WF, "ci-issue-tracker.yml");
const DAILY = join(WF, "daily-ci.yml");
const TRACKER_GROUP = "ci-issue-tracker-${{ github.repository }}";

const failures = [];
function check(cond, msg) {
  if (!cond) failures.push(msg);
}

// --- indentation-aware structural helpers -------------------------------

function load(path) {
  if (!existsSync(path)) return null;
  const text = readFileSync(path, "utf8");
  return { path, text, lines: text.split("\n") };
}

// 0 インデントの top-level key ブロックを行配列で返す。
function topBlock(lines, key) {
  const start = lines.findIndex((l) => l === `${key}:`);
  if (start === -1) return null;
  const out = [];
  for (let i = start + 1; i < lines.length; i++) {
    const l = lines[i];
    if (/^\S/.test(l)) break;
    out.push(l);
  }
  return out;
}

// `jobs:` 配下の job id(2 空白 indent)ごとの行ブロックを Map で返す。
function jobBlocks(lines) {
  const jobs = topBlock(lines, "jobs");
  if (!jobs) return null;
  const map = new Map();
  let cur = null;
  for (const l of jobs) {
    const m = l.match(/^ {2}([A-Za-z0-9_-]+):\s*$/);
    if (m) {
      cur = m[1];
      map.set(cur, []);
    } else if (cur && /^ {2}\S/.test(l)) {
      cur = null;
    } else if (cur) {
      map.get(cur).push(l);
    }
  }
  return map;
}

const indentOf = (l) => (/^\s*/.exec(l)[0] || "").length;
const blockText = (block) => (block ? block.join("\n") : "");

// `with:` ブロック内で `repository: scarecrow173/ruviform` を持つ
// checkout の `ref:` 値を返す(見つからない場合 null)。repository 行と
// 同一 indent の ref: のみを拾い、block 外の ref は拾わない。
function sharedCheckoutRef(lines) {
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/^(\s*)repository:\s*scarecrow173\/ruviform\s*$/);
    if (!m) continue;
    const indent = m[1].length;
    for (let j = i + 1; j < lines.length; j++) {
      const l = lines[j];
      if (!l.trim()) continue;
      if (indentOf(l) < indent) break; // with: ブロック終端
      const r = l.match(/^\s*ref:\s*(\S+)\s*$/);
      if (r && indentOf(l) === indent) return r[1];
    }
    return null;
  }
  return null;
}

// on: ブロック内の trigger 名(2 空白 indent の key)一覧。
function triggers(lines) {
  const on = topBlock(lines, "on");
  if (!on) return null;
  const out = [];
  for (const l of on) {
    const m = l.match(/^ {2}([A-Za-z_]+):/);
    if (m) out.push(m[1]);
  }
  return out;
}

// --- files ----------------------------------------------------------------

const rec = load(RECONCILE);
const evt = load(EVENT);
const day = load(DAILY);

check(!!rec, `${RECONCILE} が存在しない`);
check(!!evt, `${EVENT} が存在しない`);
check(!!day, `${DAILY} が存在しない`);

// --- 共有 pin 一致 ---------------------------------------------------------

for (const f of [day, evt, rec]) {
  if (!f) continue;
  const ref = sharedCheckoutRef(f.lines);
  check(
    ref === SHARED_CI_REF,
    `${f.path}: 共有実装 pin が正本と不一致(actual: ${ref ?? "none"}, expected: ${SHARED_CI_REF})`,
  );
}

// --- reconcile 入口契約 ----------------------------------------------------

if (rec) {
  const tr = triggers(rec.lines);
  check(!!tr, `${RECONCILE}: on: ブロックが解析できない`);
  if (tr) {
    check(tr.includes("schedule"), `${RECONCILE}: trigger schedule が無い`);
    check(tr.includes("workflow_dispatch"), `${RECONCILE}: trigger workflow_dispatch が無い`);
    for (const bad of ["workflow_run", "pull_request", "push"]) {
      check(
        !tr.includes(bad),
        `${RECONCILE}: trigger ${bad} が存在する(schedule/manual 入口専用契約違反)`,
      );
    }
    check(
      tr.length === 2,
      `${RECONCILE}: trigger が schedule+workflow_dispatch のみでない(actual: ${tr.join(", ")})`,
    );
  }

  const onText = blockText(topBlock(rec.lines, "on") ?? []);
  const crons = [...onText.matchAll(/cron:\s*"([^"]+)"|cron:\s*'([^']+)'|cron:\s*(\S+)/g)].map(
    (m) => m[1] ?? m[2] ?? m[3],
  );
  check(crons.length > 0, `${RECONCILE}: cron 式が無い`);
  for (const expr of crons) {
    const fields = expr.trim().split(/\s+/);
    check(fields.length === 5, `${RECONCILE}: cron 式が 5 フィールドでない(actual: ${expr})`);
    if (fields.length === 5) {
      check(
        fields[0] !== "0" && fields[0] !== "*",
        `${RECONCILE}: cron の分が毎時0分/毎分相当である(分 != 0 のずらし契約。actual: ${expr})`,
      );
    }
  }

  const conc = blockText(topBlock(rec.lines, "concurrency") ?? []);
  check(
    new RegExp(`^ {2}group:\\s*${TRACKER_GROUP.replace(/[${}]/g, "\\$&")}\\s*$`, "m").test(conc),
    `${RECONCILE}: concurrency.group が event 入口と同一でない(expected: ${TRACKER_GROUP})`,
  );
  check(
    /^ {2}cancel-in-progress:\s*false\s*$/m.test(conc),
    `${RECONCILE}: cancel-in-progress: false が無い`,
  );

  check(
    /^permissions:\s*\{\}\s*$/m.test(rec.text),
    `${RECONCILE}: workflow 直下の permissions: {} が無い`,
  );

  const jobs = jobBlocks(rec.lines);
  check(!!jobs && jobs.has("reconcile"), `${RECONCILE}: job reconcile が無い`);
  const rb = jobs?.get("reconcile");
  if (rb) {
    const jt = blockText(rb);
    const ifLine = jt.match(/^\s*if:\s*(.+)$/m);
    check(
      !!ifLine &&
        ifLine[1].includes("github.event_name == 'schedule'") &&
        ifLine[1].includes("github.ref_name == github.event.repository.default_branch"),
      `${RECONCILE}: job reconcile の if: が schedule 無条件 + dispatch ref guard 契約でない(actual: ${ifLine ? ifLine[1] : "none"})`,
    );
    check(/^\s*timeout-minutes:\s*15\s*$/m.test(jt), `${RECONCILE}: timeout-minutes: 15 が無い`);

    const perm = rb.join("\n").match(/^ {4}permissions:\s*\n((?: {6}[^\n]*\n?)*)/m);
    check(!!perm, `${RECONCILE}: job permissions が無い`);
    if (perm) {
      const keys = [...perm[1].matchAll(/^ {6}([a-z_-]+):/gm)].map((m) => m[1]).sort();
      check(
        keys.length === 3 &&
          keys.includes("actions") &&
          keys.includes("contents") &&
          keys.includes("issues"),
        `${RECONCILE}: job permissions が contents:read/actions:read/issues:write のみでない(actual: ${keys.join(", ")})`,
      );
      check(/ {6}contents:\s*read/.test(perm[1]), `${RECONCILE}: contents: read が無い`);
      check(/ {6}actions:\s*read/.test(perm[1]), `${RECONCILE}: actions: read が無い`);
      check(/ {6}issues:\s*write/.test(perm[1]), `${RECONCILE}: issues: write が無い`);
    }

    check(
      jt.includes("ref: ${{ github.event.repository.default_branch || 'main' }}"),
      `${RECONCILE}: 自 repo checkout が default branch 式 pin でない`,
    );
    check(
      jt.includes("ssh-key: ${{ secrets.RUVIFORM_DEPLOY_KEY }}"),
      `${RECONCILE}: 共有 checkout が RUVIFORM_DEPLOY_KEY を使っていない`,
    );
    check(
      (jt.match(/persist-credentials:\s*false/g) ?? []).length >= 2,
      `${RECONCILE}: 各 checkout の persist-credentials: false が不足`,
    );

    const selfIdx = rb.findIndex((l) => l.includes("node --test .ci-shared/ruviform/ci/tracker/"));
    const runIdx = rb.findIndex((l) => l.includes("tracker.mjs --reconcile"));
    check(runIdx !== -1, `${RECONCILE}: tracker.mjs --reconcile 実行が無い`);
    check(
      selfIdx !== -1 && runIdx !== -1 && selfIdx < runIdx,
      `${RECONCILE}: self-test(node --test ci/tracker)が実行 step より前に無い`,
    );
    check(
      runIdx !== -1 && rb[runIdx].includes("--report ci-issue-tracker-report.json"),
      `${RECONCILE}: --report ci-issue-tracker-report.json が無い`,
    );
    check(
      rb.some((l) => /GH_TOKEN:\s*\$\{\{\s*secrets\.GITHUB_TOKEN\s*\}\}/.test(l)),
      `${RECONCILE}: GH_TOKEN env が無い`,
    );
    check(
      jt.includes("actions/upload-artifact") && jt.includes("if: always()"),
      `${RECONCILE}: if: always() の report upload が無い`,
    );
  }

  // mode 分離: 非コメント行に TRACKER_EVENT_PATH env key を持たない。
  const noComment = rec.lines.filter((l) => !/^\s*#/.test(l));
  check(
    !noComment.some((l) => /^\s*TRACKER_EVENT_PATH\s*:/.test(l)),
    `${RECONCILE}: TRACKER_EVENT_PATH が env に設定されている(mode 分離違反)`,
  );
}

// --- event 入口不変 ---------------------------------------------------------

if (evt) {
  const tr = triggers(evt.lines);
  if (tr) {
    check(tr.includes("workflow_run"), `${EVENT}: trigger workflow_run が無い`);
    for (const bad of ["schedule", "workflow_dispatch", "pull_request", "push"]) {
      check(
        !tr.includes(bad),
        `${EVENT}: trigger ${bad} が存在する(event 入口専用契約違反 — cron は reconcile 専用ファイルに閉じ込める)`,
      );
    }
  }
  const conc = blockText(topBlock(evt.lines, "concurrency") ?? []);
  check(
    new RegExp(`^ {2}group:\\s*${TRACKER_GROUP.replace(/[${}]/g, "\\$&")}\\s*$`, "m").test(conc),
    `${EVENT}: concurrency.group が ${TRACKER_GROUP} でない`,
  );
  check(
    evt.text.includes("TRACKER_EVENT_PATH:") && !evt.text.includes("tracker.mjs --reconcile"),
    `${EVENT}: event モード入口契約が崩れている(TRACKER_EVENT_PATH 必須・--reconcile 非使用)`,
  );
}

// --- daily-ci 入口維持 ------------------------------------------------------

if (day) {
  const tr = triggers(day.lines);
  if (tr) {
    check(tr.includes("schedule"), `${DAILY}: trigger schedule が無い`);
    check(tr.includes("workflow_dispatch"), `${DAILY}: trigger workflow_dispatch が無い`);
  }
}

// --- 結果 -----------------------------------------------------------------

if (failures.length) {
  console.error(`check-ci-tracker-workflows: ${failures.length} 件の契約違反 (${ROOT})`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log(`check-ci-tracker-workflows: tracker workflow 構造契約 OK (${ROOT})`);

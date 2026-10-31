// issue-106 golden 差分の機械分類:
//   各 run の新規差分(docs/impl/ir/issue-106-baseline/<run>-vs-golden98.json)を
//   (a) issue-101-baseline/<run>-diff.json に同一 surface/path/kind が存在 → #101 既知クラス
//   (b) パスが decoder provenance (…/params/decoder/… or decoder/…) → #100 既知クラス
//   (c) #106 撤去由来: visualization/ 成果物消滅・report.html/report.data.js
//       消滅・manifest stages/artifacts 再採番・export manifest roles/
//       generator 更新 → #106 期待変更
//   (d) それ以外 → 未説明(要調査)
// に分類する。
// さらに manifest.normalized.json の集合検査: stage 集合の減少が
// "visualization" 1件のみ・artifact(role|path)集合の減少が
// visualization/report 成果物のみ・追加が 0 件であることを run ごとに
// 確認する(位置ずれ diff の機械分類では拾えない意味回帰を塞ぐ)。
import { existsSync, readdirSync, readFileSync } from "node:fs";

const DIR106 = "docs/impl/ir/issue-106-baseline";
const DIR101 = "docs/impl/ir/issue-101-baseline";
const RUNS = "baseline/runs";

const isDecoderProv = (d) =>
  /(^|\/)decoder(\/|$)/.test(d.path) || /(^|\/)decoder(\/|$)/.test(d.surface);

const isViz106 = (d, removedArtCount = 0) => {
  // visualization 成果物・静的 report の消滅。
  if (/(^|\/)visualization(\/|$)/.test(d.path)) return true;
  if (/(^|\/)visualization(\/|$)/.test(d.surface)) return true;
  if (/(^|\/)report\.(html|data\.js)$/.test(d.path)) return true;
  // 登録 artifact 除去による正規化 ART-#N 再採番:lineage 参照の
  // 値差分で、シフト量が除去件数と一致するもの。
  if (d.kind === "value" && /(^|\/)lineage(\/|$)/.test(d.path)) {
    const ma = /^ART-#(\d+)$/.exec(String(d.a));
    const mb = /^ART-#(\d+)$/.exec(String(d.b));
    if (ma && mb && Number(mb[1]) - Number(ma[1]) === removedArtCount) return true;
  }
  // manifest 系 surface の stage/artifact/role 集合・再採番。
  if (
    /manifest(\.normalized)?\.json$/.test(d.surface) &&
    /^(stages|artifacts|roles)(\/|$)/.test(d.path)
  ) {
    return true;
  }
  // export_manifest.json: roles から "visualization" 脱落・
  // generator.id が al-viz → al-dataset。
  if (/export_manifest\.json$/.test(d.surface) && /^(generator|roles)(\/|$)/.test(d.path)) {
    return true;
  }
  // export golden/snapshot.json はパッケージ内 {path: sha256} —
  // viz/report キー消失と manifest/export_manifest 内容由来の
  // hash 変化のみが起こりうる。
  if (/snapshot\.json$/.test(d.surface)) return true;
  // roles 配列等の値側の "visualization" 文字列消失。
  if (
    JSON.stringify(d.a).includes("visualization") ||
    JSON.stringify(d.b).includes("visualization")
  ) {
    return true;
  }
  return false;
};

// manifest.normalized.json の集合検査。
function checkPlan(run) {
  const oldM = JSON.parse(readFileSync(`${RUNS}/${run}-golden98/manifest.normalized.json`, "utf8"));
  const newM = JSON.parse(readFileSync(`${RUNS}/${run}/manifest.normalized.json`, "utf8"));
  const oldStages = new Set(oldM.stages.map((s) => s.stage));
  const newStages = new Set(newM.stages.map((s) => s.stage));
  const removedStages = [...oldStages].filter((s) => !newStages.has(s));
  const addedStages = [...newStages].filter((s) => !oldStages.has(s));
  // 追加は常に 0 件・除去は visualization のみ(full pipeline run)か
  // 0 件(--stages ...,dataset 指定で元々 plan に無かった run)。
  const stageOk = addedStages.length === 0 && removedStages.every((s) => s === "visualization");

  const key = (a) => `${a.role}|${a.path}`;
  const oldKeys = new Set(oldM.artifacts.map(key));
  const newKeys = new Set(newM.artifacts.map(key));
  const removedArts = oldM.artifacts.filter((a) => !newKeys.has(key(a)));
  const addedArts = newM.artifacts.filter((a) => !oldKeys.has(key(a)));
  const artOk =
    addedArts.length === 0 &&
    removedArts.every(
      (a) =>
        a.role === "visualization" ||
        /(^|\/)visualization(\/|$)/.test(a.path) ||
        /(^|\/)report\.(html|data\.js)$/.test(a.path),
    );
  return { stageOk, artOk, removedStages, addedStages, removedArts, addedArts };
}

const results = {};
let violations = 0;
for (const f of readdirSync(DIR106)) {
  const run = f.replace(/-vs-golden98\.json$/, "");
  if (run === f || !existsSync(`${RUNS}/${run}-golden98`)) continue;
  const diffs = JSON.parse(readFileSync(`${DIR106}/${f}`, "utf8"));
  let base101 = [];
  try {
    base101 = JSON.parse(readFileSync(`${DIR101}/${run}-diff.json`, "utf8"));
  } catch {
    /* run not present in #101 capture */
  }
  const keys101 = new Set(base101.map((d) => `${d.surface}|${d.path}|${d.kind}`));
  const plan = checkPlan(run);
  const out = { cls101: 0, cls100: 0, cls106: 0, unexplained: [] };
  for (const d of diffs) {
    if (keys101.has(`${d.surface}|${d.path}|${d.kind}`)) out.cls101++;
    else if (isDecoderProv(d)) out.cls100++;
    else if (isViz106(d, plan.removedArts.length)) out.cls106++;
    else out.unexplained.push(d);
  }
  if (!plan.stageOk || !plan.artOk) violations++;
  results[run] = {
    total: diffs.length,
    ...out,
    unexplained_count: out.unexplained.length,
    plan_check: plan.stageOk && plan.artOk ? "ok" : plan,
  };
  console.log(
    `${run}: total=${diffs.length} #101=${out.cls101} #100=${out.cls100} ` +
      `#106=${out.cls106} unexplained=${out.unexplained.length} ` +
      `plan=${plan.stageOk && plan.artOk ? "ok" : "VIOLATION"}`,
  );
  for (const u of out.unexplained.slice(0, 10)) {
    console.log(`   UNEXPLAINED: ${JSON.stringify(u).slice(0, 260)}`);
  }
  if (!plan.stageOk || !plan.artOk) {
    console.log(`   PLAN VIOLATION: ${JSON.stringify(plan).slice(0, 400)}`);
  }
}
process.exitCode =
  Object.values(results).some((r) => r.unexplained_count > 0) || violations > 0 ? 1 : 0;

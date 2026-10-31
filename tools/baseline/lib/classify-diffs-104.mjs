// issue-104 golden 差分の機械分類:
//   各 run の新規差分(docs/impl/ir/issue-104-baseline/<run>-vs-golden98.json)を
//   (a) issue-101-baseline/<run>-diff.json に同一 surface/path/kind が存在 → #101 既知クラス
//   (b) decoder version 系差分の a 側(新 capture)が Cargo.lock 導出 rev と
//       一致 → #104 期待変更(RUVIFORM_REV の Cargo.lock 導出化。
//       #100 より先に判定して rev 伝播を値レベルで検証する)
//   (c) パスが decoder provenance (…/params/decoder/… or decoder/…) → #100 既知クラス
//   (d) #106 撤去由来: visualization/ 成果物消滅・report.html/report.data.js
//       消滅・manifest stages/artifacts 再採番・export manifest roles/
//       generator 更新 → #106 期待変更
//   (e) それ以外 → 未説明(要調査)
// に分類する。
// さらに manifest.normalized.json の集合検査: stage 集合の減少が
// "visualization" 1件のみ・artifact(role|path)集合の減少が
// visualization/report 成果物のみ・追加が 0 件であることを run ごとに
// 確認する。加えて新 capture の metadata golden から
// decoder.version / decoder.settings.ruviform.rev が Cargo.lock 導出 rev と
// 一致することを直接検証する(#115 解消の証跡)。
import { existsSync, readdirSync, readFileSync } from "node:fs";

const DIR104 = "docs/impl/ir/issue-104-baseline";
const DIR101 = "docs/impl/ir/issue-101-baseline";
const RUNS = "baseline/runs";

// Cargo.lock から ruviform git source の解決済み rev を導出する。
// build.rs(RUVIFORM_REV)と同じ情報源を使い、ハードコードを持ち込まない。
function lockfileRuviformRev() {
  const lock = readFileSync("Cargo.lock", "utf8");
  const revs = new Set(
    [...lock.matchAll(/ruviform\.git\?rev=([0-9a-f]{40})#([0-9a-f]{40})/g)].map((m) => {
      if (m[1] !== m[2]) {
        throw new Error(`rev/pinned sha mismatch in Cargo.lock: ${m[1]} != ${m[2]}`);
      }
      return m[1];
    }),
  );
  if (revs.size !== 1) {
    throw new Error(`expected single ruviform rev in Cargo.lock, got ${[...revs]}`);
  }
  return [...revs][0];
}
const LOCK_REV = lockfileRuviformRev();
console.log(`lockfile ruviform rev: ${LOCK_REV}`);

const isDecoderProv = (d) =>
  /(^|\/)decoder(\/|$)/.test(d.path) || /(^|\/)decoder(\/|$)/.test(d.surface);

// #104: decoder version 系差分で新値が lockfile rev と一致するもの。
// (旧 capture では a=e467ccb… ハードコードだったのが f3f474b… 導出値へ変化)
const isRev104 = (d) => d.kind === "value" && isDecoderProv(d) && d.a === LOCK_REV;

const isViz106 = (d, removedArtCount = 0) => {
  if (/(^|\/)visualization(\/|$)/.test(d.path)) return true;
  if (/(^|\/)visualization(\/|$)/.test(d.surface)) return true;
  if (/(^|\/)report\.(html|data\.js)$/.test(d.path)) return true;
  if (d.kind === "value" && /(^|\/)lineage(\/|$)/.test(d.path)) {
    const ma = /^ART-#(\d+)$/.exec(String(d.a));
    const mb = /^ART-#(\d+)$/.exec(String(d.b));
    if (ma && mb && Number(mb[1]) - Number(ma[1]) === removedArtCount) return true;
  }
  if (
    /manifest(\.normalized)?\.json$/.test(d.surface) &&
    /^(stages|artifacts|roles)(\/|$)/.test(d.path)
  ) {
    return true;
  }
  if (/export_manifest\.json$/.test(d.surface) && /^(generator|roles)(\/|$)/.test(d.path)) {
    return true;
  }
  if (/snapshot\.json$/.test(d.surface)) return true;
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

// 新 capture の golden metadata から decoder rev の直接検証。
// 返り値: { checked, bad: [...] }
function checkDecoderRev(run) {
  const dir = `${RUNS}/${run}/golden/inputs`;
  const bad = [];
  let checked = 0;
  if (!existsSync(dir)) return { checked, bad };
  for (const inDir of readdirSync(dir)) {
    const f = `${dir}/${inDir}/metadata.json`;
    if (!existsSync(f)) continue;
    const meta = JSON.parse(readFileSync(f, "utf8"));
    const dec = meta.decoder ?? {};
    checked++;
    if (dec.backend !== "ruviform") bad.push(`${f}: backend=${dec.backend}`);
    if (dec.version !== LOCK_REV) bad.push(`${f}: version=${dec.version}`);
    const rev = dec.settings?.ruviform?.rev;
    if (rev !== LOCK_REV) bad.push(`${f}: settings.ruviform.rev=${rev}`);
    for (const [i, at] of (dec.attempts ?? []).entries()) {
      if (at.version !== LOCK_REV) bad.push(`${f}: attempts[${i}].version=${at.version}`);
    }
  }
  return { checked, bad };
}

const results = {};
let violations = 0;
for (const f of readdirSync(DIR104)) {
  const run = f.replace(/-vs-golden98\.json$/, "");
  if (run === f || !existsSync(`${RUNS}/${run}-golden98`)) continue;
  const diffs = JSON.parse(readFileSync(`${DIR104}/${f}`, "utf8"));
  let base101 = [];
  try {
    base101 = JSON.parse(readFileSync(`${DIR101}/${run}-diff.json`, "utf8"));
  } catch {
    /* run not present in #101 capture */
  }
  const keys101 = new Set(base101.map((d) => `${d.surface}|${d.path}|${d.kind}`));
  const plan = checkPlan(run);
  const revCheck = checkDecoderRev(run);
  const out = { cls101: 0, cls104: 0, cls100: 0, cls106: 0, unexplained: [] };
  for (const d of diffs) {
    if (keys101.has(`${d.surface}|${d.path}|${d.kind}`)) out.cls101++;
    else if (isRev104(d)) out.cls104++;
    else if (isDecoderProv(d)) out.cls100++;
    else if (isViz106(d, plan.removedArts.length)) out.cls106++;
    else out.unexplained.push(d);
  }
  if (!plan.stageOk || !plan.artOk) violations++;
  if (revCheck.bad.length > 0 || revCheck.checked === 0) violations++;
  results[run] = {
    total: diffs.length,
    ...out,
    unexplained_count: out.unexplained.length,
    plan_check: plan.stageOk && plan.artOk ? "ok" : plan,
    rev_check: revCheck.bad.length === 0 ? `ok(${revCheck.checked} inputs)` : revCheck.bad,
  };
  console.log(
    `${run}: total=${diffs.length} #101=${out.cls101} #104=${out.cls104} ` +
      `#100=${out.cls100} #106=${out.cls106} unexplained=${out.unexplained.length} ` +
      `plan=${plan.stageOk && plan.artOk ? "ok" : "VIOLATION"} ` +
      `rev=${revCheck.bad.length === 0 ? `ok(${revCheck.checked})` : "VIOLATION"}`,
  );
  for (const u of out.unexplained.slice(0, 10)) {
    console.log(`   UNEXPLAINED: ${JSON.stringify(u).slice(0, 260)}`);
  }
  for (const b of revCheck.bad.slice(0, 10)) {
    console.log(`   REV VIOLATION: ${b}`);
  }
  if (!plan.stageOk || !plan.artOk) {
    console.log(`   PLAN VIOLATION: ${JSON.stringify(plan).slice(0, 400)}`);
  }
}
process.exitCode =
  Object.values(results).some((r) => r.unexplained_count > 0) || violations > 0 ? 1 : 0;

// issue-103 golden 差分の機械分類:
//   各 run の新規差分(docs/impl/ir/issue-103-baseline/<run>-vs-golden98.json)を
//   (a) issue-101-baseline/<run>-diff.json に同一 surface/path/kind が存在 → #101 既知クラス
//   (b) パスが decoder provenance (…/params/decoder/… or decoder/…) → #100 既知クラス
//   (c) それ以外 → 未説明(要調査)
// に分類する。
import { readdirSync, readFileSync } from "node:fs";

const DIR103 = "docs/impl/ir/issue-103-baseline";
const DIR101 = "docs/impl/ir/issue-101-baseline";

const isDecoderProv = (d) =>
  /(^|\/)decoder(\/|$)/.test(d.path) || /(^|\/)decoder(\/|$)/.test(d.surface);

const results = {};
for (const f of readdirSync(DIR103)) {
  const run = f.replace(/-vs-golden98\.json$/, "");
  const diffs = JSON.parse(readFileSync(`${DIR103}/${f}`, "utf8"));
  let base101 = [];
  try {
    base101 = JSON.parse(readFileSync(`${DIR101}/${run}-diff.json`, "utf8"));
  } catch {
    /* run not present in #101 capture */
  }
  const keys101 = new Set(base101.map((d) => `${d.surface}|${d.path}|${d.kind}`));
  const out = { cls101: 0, cls100: 0, unexplained: [] };
  for (const d of diffs) {
    if (keys101.has(`${d.surface}|${d.path}|${d.kind}`)) out.cls101++;
    else if (isDecoderProv(d)) out.cls100++;
    else out.unexplained.push(d);
  }
  results[run] = { total: diffs.length, ...out, unexplained_count: out.unexplained.length };
  console.log(
    `${run}: total=${diffs.length} #101=${out.cls101} #100=${out.cls100} unexplained=${out.unexplained.length}`,
  );
  for (const u of out.unexplained.slice(0, 10)) {
    console.log(`   UNEXPLAINED: ${JSON.stringify(u).slice(0, 260)}`);
  }
}

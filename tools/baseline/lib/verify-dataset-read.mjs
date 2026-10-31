// headless Dataset 読み出し検証: 引数 = <jobDir>/analysis
// table/tensor/audio/JSON を GUI 層非依存で読み出せることを確認する。
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { findTablesAndTensors, readTable, readTensor, readWav, tableToRows } from "./store.mjs";

const jd = process.argv[2];
if (!jd) {
  console.error("usage: node verify-dataset-read.mjs <analysis dir>");
  process.exit(2);
}

function* walk(dir, rel = "") {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const r = rel ? `${rel}/${e.name}` : e.name;
    if (e.isDirectory()) yield* walk(path.join(dir, e.name), r);
    else yield r;
  }
}
const walkFiles = async (dir) => Array.from(walk(dir));

const { tables, tensors } = await findTablesAndTensors(jd, walkFiles);
console.log("== tables ==");
for (const t of tables) {
  const tab = await readTable(path.join(jd, t));
  const rows = tableToRows(tab);
  console.log(`${t}: rows=${rows.length} cols=[${Object.keys(tab.columns).join(",")}]`);
  if (rows[0]) console.log(`  row0: ${JSON.stringify(rows[0]).slice(0, 400)}`);
  if (rows.length > 1)
    console.log(`  rowN: ${JSON.stringify(rows[rows.length - 1]).slice(0, 400)}`);
}

console.log("== tensors (region read: first chunk) ==");
for (const t of tensors) {
  const { meta, data } = await readTensor(path.join(jd, t));
  const preview = Array.from(data.slice(0, 4)).map((v) =>
    typeof v === "object" ? JSON.stringify(v) : v,
  );
  console.log(
    `${t}: shape=${JSON.stringify(meta.shape)} chunks=${meta.chunks?.length ?? "?"} n=${data.length} first=${preview.join(",")}`,
  );
}

console.log("== audio (wav) ==");
for (const rel of walk(jd)) {
  if (!rel.endsWith(".wav")) continue;
  const { fmt, samples } = await readWav(path.join(jd, rel));
  console.log(
    `${rel}: ${fmt} frames=${samples.length} first=${Array.from(samples.slice(0, 3)).join(",")}`,
  );
}

console.log("== json artifacts ==");
const manifest = JSON.parse(readFileSync(path.join(jd, "manifest.json"), "utf8"));
console.log(
  `manifest: schema=${manifest.schema_version} inputs=${(manifest.inputs ?? []).length} roles=${Object.keys(manifest.roles ?? manifest.stores ?? {}).join(",")}`,
);
for (const rel of walk(jd)) {
  if (rel.endsWith(".json") && rel !== "manifest.json" && !rel.includes("/x.bin"))
    console.log(`  ${rel}`);
}

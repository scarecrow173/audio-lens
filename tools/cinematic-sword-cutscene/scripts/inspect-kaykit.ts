// Surveys the KayKit GLBs (animations, durations, bones, attachments) and
// verifies src/data/kaykitManifest.ts against the real files.
// Run: node scripts/inspect-kaykit.ts
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..");

type GltfJson = {
  nodes?: { name?: string; children?: number[]; mesh?: number }[];
  meshes?: { name?: string }[];
  animations?: { name?: string; samplers?: { input: number }[] }[];
  accessors?: { max?: number[] }[];
  skins?: unknown[];
};

const readGlbJson = (path: string): GltfJson => {
  const buf = readFileSync(path);
  const jsonLen = buf.readUInt32LE(12);
  return JSON.parse(buf.subarray(20, 20 + jsonLen).toString("utf8"));
};

for (const file of ["Rogue.glb", "Knight.glb"]) {
  const g = readGlbJson(join(root, "public/models", file));
  const nodes = g.nodes ?? [];
  const anims = g.animations ?? [];
  const accessors = g.accessors ?? [];
  console.log(`=== ${file} ===`);
  console.log(`nodes=${nodes.length} skins=${(g.skins ?? []).length}`);

  // attachment slots
  for (const n of nodes) {
    if (!n.name?.startsWith("handslot")) continue;
    const kids = (n.children ?? []).map((c) => nodes[c]?.name).join(", ");
    console.log(`  ${n.name} -> ${kids}`);
  }

  // clip table (name + authored duration from sampler input max)
  const table = anims.map((a) => {
    let max = 0;
    for (const s of a.samplers ?? []) {
      const m = accessors[s.input]?.max?.[0];
      if (m && m > max) max = m;
    }
    return { name: a.name ?? "?", duration: max };
  });
  for (const c of table) console.log(`  clip ${c.name} ${c.duration.toFixed(3)}s`);

  // verify manifest
  const manifestId = file.startsWith("Rogue") ? "rogue" : "knight";
  const { KAYKIT } = await import("../src/data/kaykitManifest.ts");
  const mf = KAYKIT[manifestId as "rogue" | "knight"];
  const missing = table.filter((c) => !mf.clips.some((m) => m.name === c.name));
  const extra = mf.clips.filter((m) => !table.some((c) => c.name === m.name));
  const durDiff = table
    .map((c) => {
      const m = mf.clips.find((x) => x.name === c.name);
      return m && Math.abs(m.duration - c.duration) > 0.01
        ? `${c.name}: manifest=${m.duration} actual=${c.duration.toFixed(3)}`
        : null;
    })
    .filter(Boolean);
  if (missing.length || extra.length || durDiff.length) {
    console.log("  MANIFEST DRIFT:");
    missing.forEach((c) => {
      console.log(`    missing in manifest: ${c.name}`);
    });
    extra.forEach((c) => {
      console.log(`    extra in manifest: ${c.name}`);
    });
    durDiff.forEach((d) => {
      console.log(`    duration diff: ${d}`);
    });
  } else {
    console.log("  manifest OK");
  }
}

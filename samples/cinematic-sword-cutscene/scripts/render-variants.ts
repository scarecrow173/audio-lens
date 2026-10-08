// Renders the demo variants and writes machine-readable metadata.
// Run: node scripts/render-variants.ts [variantId ...]
//      (no args renders all variants)
import {execSync} from 'node:child_process';
import {mkdirSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {AUDIO_BUG_VARIANTS} from '../src/bugs/audioBugs.ts';

const root = join(fileURLToPath(new URL('.', import.meta.url)), '..');
// Fixture outputs live in the samples/<scenario>/ convention dir next to
// this generator project, so consumers only need samples/game-capture/.
const OUT_ROOT = join(root, '..', 'game-capture');
const FPS = 30;

type Variant = {
  id: string;
  composition: string;
  out: string;
  durationFrames: number;
  bug?: Record<string, unknown>;
  audioBug?: Record<string, unknown>;
};

const VARIANTS: Variant[] = [
  {
    id: 'test',
    composition: 'GameCaptureTest',
    out: 'test/game-capture-test.mp4',
    durationFrames: 300,
  },
  {
    id: 'normal',
    composition: 'GameCaptureNormal',
    out: 'normal/game-capture-normal.mp4',
    durationFrames: 1500,
  },
  // Audio bug variants: identical picture, transformed soundtrack.
  ...AUDIO_BUG_VARIANTS.map((v) => ({
    id: v.id,
    composition: v.composition,
    out: `bugs/game-capture-${v.id}.mp4`,
    durationFrames: 1500,
    audioBug: v.bug as unknown as Record<string, unknown>,
  })),
];

const wanted = process.argv.slice(2);
const targets = wanted.length
  ? VARIANTS.filter((v) => wanted.includes(v.id))
  : VARIANTS;

if (targets.length === 0) {
  console.error(`Unknown variant(s): ${wanted.join(', ')}`);
  process.exit(1);
}

mkdirSync(join(OUT_ROOT, 'metadata'), {recursive: true});

for (const v of targets) {
  const outPath = join(OUT_ROOT, v.out);
  console.log(`\n=== render ${v.id} -> ${outPath} ===`);
  execSync(`npx remotion render ${v.composition} "${outPath}"`, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32' ? 'cmd.exe' : undefined,
  });
  const meta: Record<string, unknown> = {
    variant: v.id === 'test' ? 'test' : v.bug ? (v.bug.type as string) : v.audioBug ? (v.audioBug.type as string) : 'normal',
    width: 640,
    height: 360,
    fps: FPS,
    durationFrames: v.durationFrames,
    composition: v.composition,
    file: `game-capture/${v.out}`,
  };
  if (v.bug) meta.bug = v.bug;
  if (v.audioBug) meta.audioBug = v.audioBug;
  const metaPath = join(OUT_ROOT, 'metadata', `${v.id}.json`);
  writeFileSync(metaPath, JSON.stringify(meta, null, 2) + '\n');
  console.log(`metadata -> ${metaPath}`);
}

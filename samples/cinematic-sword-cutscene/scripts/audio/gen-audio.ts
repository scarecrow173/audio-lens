// Generates all procedural audio (sfx + ambience + bgm + bug mixes) and
// writes samples/game-capture/metadata/audio.json by merging generation
// info with the placed audio events from src/data/audioTimeline.ts.
//
// Voice WAVs are produced separately by gen_voice.py (Chatterbox);
// this script reads their WAV headers for durations. Missing voice
// files are tolerated — their events are still recorded.
//
// Run: npx tsx scripts/audio/gen-audio.ts
import {readFileSync, existsSync, mkdirSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {generateBgm} from './gen-bgm.ts';
import {generateBugMixes} from './gen-bug-audio.ts';
import {type GenInfo, SFX_REGISTRY, generateSfx} from './gen-sfx.ts';
import {AUDIO_MASTER, AUDIO_TEST, type AudioEvent} from '../../src/data/audioTimeline.ts';
import {AUDIO_BUG_VARIANTS, audioMixFile} from '../../src/bugs/audioBugs.ts';

const root = join(fileURLToPath(new URL('.', import.meta.url)), '../..');
const FPS = 30;

// PCM16 stereo WAV header -> duration seconds
const wavDuration = (path: string): number | null => {
  if (!existsSync(path)) return null;
  const b = readFileSync(path);
  const byteRate = b.readUInt32LE(28);
  const dataSize = b.readUInt32LE(40);
  return byteRate > 0 ? dataSize / byteRate : null;
};

// Voice generation info — mirrors gen_voice.py settings.
const VOICE_INFO: Record<string, Omit<GenInfo, 'file' | 'durationSec'>> = {
  kael_01: {generator: 'chatterbox-mtl-0.1.7+dsp(bright-dry)', seed: 7001},
  kael_02: {generator: 'chatterbox-mtl-0.1.7+dsp(bright-dry)', seed: 7003},
  varen_01: {generator: 'chatterbox-mtl-0.1.7+dsp(dark-reverb)', seed: 7002},
  varen_02: {generator: 'chatterbox-mtl-0.1.7+dsp(dark-reverb)', seed: 7004},
  varen_03: {generator: 'chatterbox-mtl-0.1.7+dsp(dark-reverb)', seed: 7005},
};

generateSfx(root);
const bgm = generateBgm(root);
generateBugMixes(root);

const genByFile = new Map<string, GenInfo>();
for (const g of SFX_REGISTRY) genByFile.set(g.file, g);
genByFile.set(bgm.file, bgm);

const toMeta = (e: AudioEvent) => {
  const voiceId = e.file.split('/').pop()?.replace('.wav', '') ?? '';
  const gen =
    genByFile.get(e.file) ??
    (VOICE_INFO[voiceId]
      ? {...VOICE_INFO[voiceId], durationSec: e.durationSec}
      : {generator: 'unknown', seed: -1, durationSec: e.durationSec});
  const realDur = wavDuration(join(root, 'assets', e.file));
  return {
    id: e.id,
    type: e.type,
    filename: e.file,
    startFrame: Math.round(e.startSec * FPS),
    durationSec: +(realDur ?? e.durationSec).toFixed(3),
    volume: e.volume,
    speaker: e.speaker ?? null,
    generator: gen.generator,
    seed: gen.seed,
    loop: e.loop ?? false,
  };
};

const meta = {
  fps: FPS,
  timeline: 'master',
  events: AUDIO_MASTER.map(toMeta),
  testTimeline: 'test',
  testEvents: AUDIO_TEST.map(toMeta),
  masterMix: 'audio/generated/master/mix-normal.wav',
  audioBugVariants: AUDIO_BUG_VARIANTS.map((v) => ({
    id: v.id,
    file: audioMixFile(v.id),
    bug: v.bug,
    description: v.description,
  })),
};

// Metadata lands with the fixture outputs (samples/game-capture/).
const metaDir = join(root, '..', 'game-capture', 'metadata');
mkdirSync(metaDir, {recursive: true});
writeFileSync(
  join(metaDir, 'audio.json'),
  JSON.stringify(meta, null, 2) + '\n'
);
console.log(`wrote game-capture/metadata/audio.json (${meta.events.length} master events)`);

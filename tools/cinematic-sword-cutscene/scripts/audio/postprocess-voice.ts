// Character voice post-processing — deterministic DSP (no ffmpeg needed).
// KAEL: slightly brighter + dry/calm.  VAREN: darker, low-weight, room.
//
// Run: npx tsx scripts/audio/postprocess-voice.ts
import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  compressor,
  highpass,
  lowShelf,
  mkBuf,
  normalize,
  peakingEq,
  readWav,
  resampleRate,
  room,
  SR,
  softClip,
  trimSilence,
  writeWav,
} from "./dsp.ts";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "../..");
const RAW = `${root}/assets/audio/generated/voice/raw`;
const OUT = `${root}/assets/audio/generated/voice`;

type Chain = (m: Float32Array) => Float32Array;

// mono chain; stereo widening happens after
const kaelChain: Chain = (m) => {
  // +~0.5 semitone tape shift (also resamples 24k -> 44.1k)
  let v = resampleRate(m, (24000 / SR) * 1.035);
  v = trimSilence(v, -42, 0.07);
  highpass(v, 110);
  peakingEq(v, 3200, 1.2, 3); // presence
  compressor(v, -18, 3, 6, 120, 3);
  softClip(v, 1.15); // very light saturation
  v = room(v, 0.12, 0.22, 0.55); // small dry room
  return v;
};

const varenChain: Chain = (m) => {
  // -~2.8 semitones: darker, heavier, slightly slower
  let v = resampleRate(m, (24000 / SR) * 0.85);
  v = trimSilence(v, -42, 0.09);
  highpass(v, 70);
  lowShelf(v, 160, 5); // low-frequency weight
  peakingEq(v, 900, 1.1, -2.5); // scoop nasal mids
  compressor(v, -20, 4, 8, 160, 4);
  softClip(v, 1.3);
  v = room(v, 0.3, 0.38, 0.4); // bigger room
  return v;
};

const LINES: { id: string; chain: Chain }[] = [
  { id: "kael_01", chain: kaelChain },
  { id: "kael_02", chain: kaelChain },
  { id: "varen_01", chain: varenChain },
  { id: "varen_02", chain: varenChain },
  { id: "varen_03", chain: varenChain },
];

let missing = 0;
for (const { id, chain } of LINES) {
  const src = `${RAW}/${id}.wav`;
  if (!existsSync(src)) {
    console.warn(`missing ${src} — run gen_voice.py first`);
    missing++;
    continue;
  }
  const { buf } = readWav(src);
  const mono = buf.l; // chatterbox output is mono
  const v = chain(mono);
  const st = mkBuf(v.length / SR);
  // subtle stereo decorrelation
  const off = Math.round(0.004 * SR);
  for (let i = 0; i < v.length; i++) {
    st.l[i] = v[i];
    st.r[i] = i >= off ? v[i - off] * 0.96 : v[i];
  }
  normalize(st, 0.82);
  writeWav(st, `${OUT}/${id}.wav`);
  console.log(`voice -> ${id}.wav (${(v.length / SR).toFixed(2)}s)`);
}

process.exit(missing > 0 ? 1 : 0);

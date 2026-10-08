// Premixes the master audio timeline into one deterministic WAV
// (assets/audio/generated/master/mix-normal.wav), then renders every
// audio-bug variant by applying a deterministic DSP transform to that
// premix. All source WAVs are generated at SR (44.1kHz), so no
// resampling is needed and mixes stay bit-exact across runs.
//
// The mixing mirrors AudioTrack.tsx semantics:
//   non-loop event: plays min(file length, durationSec) from startSec
//   loop event:     repeated until the timeline end
//   volume:         linear gain per event
import {existsSync, mkdirSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {AUDIO_MASTER} from '../../src/data/audioTimeline.ts';
import {
  AUDIO_BUG_VARIANTS,
  audioMixFile,
  type AudioBugProfile,
} from '../../src/bugs/audioBugs.ts';
import {
  SR,
  bandpass,
  highpass,
  lowpass,
  mkBuf,
  mixInto,
  readWav,
  rng,
  room,
  writeWav,
  type Stereo,
} from './dsp.ts';

export const MASTER_DURATION_SEC = 50;

// Reference premix of the normal soundtrack.
export const mixMaster = (root: string): Stereo => {
  const mix = mkBuf(MASTER_DURATION_SEC);
  const missing: string[] = [];
  for (const e of AUDIO_MASTER) {
    const p = join(root, 'assets', e.file);
    if (!existsSync(p)) {
      missing.push(e.file);
      continue;
    }
    const {buf, sr} = readWav(p);
    if (sr !== SR) {
      throw new Error(`${e.file}: expected ${SR}Hz, got ${sr}Hz`);
    }
    if (e.loop) {
      const fileDur = buf.l.length / SR;
      for (let at = e.startSec; at < MASTER_DURATION_SEC; at += fileDur) {
        mixInto(mix, buf, at, e.volume);
      }
    } else {
      const n = Math.min(buf.l.length, Math.round(e.durationSec * SR));
      mixInto(mix, {l: buf.l.subarray(0, n), r: buf.r.subarray(0, n)}, e.startSec, e.volume);
    }
  }
  if (missing.length) console.warn(`missing sources skipped: ${missing.join(', ')}`);
  return mix;
};

const zeroRange = (buf: Stereo, fromSec: number, toSec: number): void => {
  const s = Math.max(0, Math.round(fromSec * SR));
  const e = Math.min(buf.l.length, Math.round(toSec * SR));
  buf.l.fill(0, s, e);
  buf.r.fill(0, s, e);
};

const shifted = (buf: Stereo, offsetSec: number): Stereo => {
  const out = mkBuf(buf.l.length / SR);
  const shift = Math.round(offsetSec * SR);
  for (let i = 0; i < buf.l.length; i++) {
    const dst = i + shift;
    if (dst >= 0 && dst < out.l.length) {
      out.l[dst] = buf.l[i];
      out.r[dst] = buf.r[i];
    }
  }
  return out;
};

const hardClipGain = (buf: Stereo, gainDb: number): Stereo => {
  const g = 10 ** (gainDb / 20);
  for (const ch of [buf.l, buf.r]) {
    for (let i = 0; i < ch.length; i++) {
      ch[i] = Math.max(-0.99, Math.min(0.99, ch[i] * g));
    }
  }
  return buf;
};

const addClicks = (
  buf: Stereo,
  seed: number,
  count: number,
  fromSec: number,
  toSec: number
): Stereo => {
  const rand = rng(seed);
  const burstLen = Math.round(0.0008 * SR);
  for (let k = 0; k < count; k++) {
    const at = Math.round((fromSec + rand() * (toSec - fromSec)) * SR);
    const amp = (0.4 + rand() * 0.55) * (rand() < 0.5 ? -1 : 1);
    for (let i = 0; i < burstLen && at + i < buf.l.length; i++) {
      const env = Math.exp(-i / (burstLen / 3));
      buf.l[at + i] += amp * env;
      buf.r[at + i] += amp * env;
    }
  }
  return buf;
};

export const applyAudioBug = (mix: Stereo, bug: AudioBugProfile): Stereo => {
  switch (bug.type) {
    case 'dropout':
      zeroRange(mix, bug.startSec, bug.endSec);
      return mix;
    case 'desync':
      return shifted(mix, bug.offsetSec);
    case 'clipping':
      return hardClipGain(mix, bug.gainDb);
    case 'reverb':
      return {
        l: room(mix.l, bug.wet, bug.decaySec, 0.25),
        r: room(mix.r, bug.wet, bug.decaySec, 0.25),
      };
    case 'filter': {
      const f =
        bug.kind === 'lowpass'
          ? (x: Float32Array) => lowpass(x, bug.cutoffHz, 2)
          : bug.kind === 'highpass'
            ? (x: Float32Array) => highpass(x, bug.cutoffHz)
            : (x: Float32Array) => bandpass(x, bug.cutoffHz, bug.q ?? 1);
      return {l: f(mix.l), r: f(mix.r)};
    }
    case 'clicks':
      return addClicks(mix, bug.seed, bug.count, bug.startSec, bug.endSec);
  }
};

export const generateBugMixes = (root: string): void => {
  const dir = join(root, 'assets/audio/generated/master');
  mkdirSync(dir, {recursive: true});
  const mix = mixMaster(root);
  // Defensive copy per variant: transforms may mutate in place.
  const copy = (): Stereo => ({l: mix.l.slice(), r: mix.r.slice()});
  writeWav(mix, join(dir, 'mix-normal.wav'));
  for (const v of AUDIO_BUG_VARIANTS) {
    writeWav(applyAudioBug(copy(), v.bug), join(dir, `mix-${v.id}.wav`));
    console.log(`bug mix -> ${audioMixFile(v.id)} (${v.description})`);
  }
};

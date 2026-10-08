// Procedural SFX + ambience generation. No sampled assets, no AI —
// every file is synthesized from seeded oscillators/noise/filters.
//
// Output: assets/audio/generated/sfx/, assets/audio/generated/ambience/
import {
  SR,
  type Stereo,
  addNoise,
  addOsc,
  adsr,
  bandpass,
  expDecay,
  lowShelf,
  lowpass,
  mkBuf,
  mkMono,
  mixInto,
  mixMono,
  normalize,
  rng,
  room,
  softClip,
  toStereo,
  writeWav,
} from './dsp.ts';

export type GenInfo = {
  file: string; // path relative to assets/
  generator: string;
  seed: number;
  durationSec: number;
};

export const SFX_REGISTRY: GenInfo[] = [];

const SFX_DIR = 'assets/audio/generated/sfx';
const AMB_DIR = 'assets/audio/generated/ambience';

const reg = (file: string, generator: string, seed: number, durationSec: number) => {
  SFX_REGISTRY.push({file, generator, seed, durationSec});
};

// ---------- helpers ----------

// Metallic hit: inharmonic partial cluster + noise attack.
const metallic = (
  dur: number,
  seed: number,
  base: number,
  partials: number[],
  noiseGain: number,
  ringDecay: number
): Stereo => {
  const rand = rng(seed);
  const buf = mkMono(dur);
  for (const p of partials) {
    const f = base * p * (1 + (rand() - 0.5) * 0.01);
    addOsc(buf, 'sine', 0, dur, f, 0.5 / partials.length, expDecay(ringDecay * (0.6 + p * 0.12)));
  }
  addNoise(buf, rand, 0, 0.05, noiseGain, expDecay(0.28));
  return toStereo(softClip(buf, 1.6));
};

// Noise whoosh with swept band-pass centre.
const whoosh = (
  dur: number,
  seed: number,
  f0: number,
  f1: number,
  gain: number
): Stereo => {
  const rand = rng(seed);
  const raw = mkMono(dur);
  addNoise(raw, rand, 0, dur, gain, adsr(0.35, 0.15, 0.9, 0.3));
  // swept resonant emphasis: mix two band-passed copies
  const lo = Float32Array.from(raw);
  const hi = Float32Array.from(raw);
  bandpass(lo, (f0 + f1) / 2, 1.6);
  bandpass(hi, Math.max(f1 * 0.8, 60), 2.4);
  for (let i = 0; i < raw.length; i++) {
    const u = i / raw.length;
    raw[i] = lo[i] * (1 - u) + hi[i] * u + raw[i] * 0.18;
  }
  lowpass(raw, f1 * 1.6);
  return toStereo(raw);
};

// ---------- SFX ----------

const sfxSwingLight = (): Stereo => {
  const b = whoosh(0.32, 101, 500, 3400, 0.7);
  const m = mkMono(0.32);
  addOsc(m, 'sine', 0.02, 0.26, (u) => 900 + 2400 * u, 0.05, adsr(0.4, 0.2, 0.6, 0.4));
  mixInto(b, toStereo(m), 0, 1);
  return b;
};

const sfxSwingHeavy = (): Stereo => {
  const b = whoosh(0.48, 102, 180, 1100, 0.9);
  const m = mkMono(0.48);
  addOsc(m, 'tri', 0, 0.42, (u) => 160 + 320 * u, 0.16, adsr(0.3, 0.2, 0.7, 0.4));
  mixInto(b, toStereo(m), 0, 1);
  lowShelf(b.l, 250, 6);
  lowShelf(b.r, 250, 6);
  return b;
};

const sfxClash = (): Stereo => {
  const b = metallic(0.75, 103, 780, [1, 1.51, 1.98, 2.57, 3.31], 0.5, 1.4);
  const m = mkMono(0.75);
  addOsc(m, 'sine', 0, 0.12, 220, 0.5, expDecay(0.5));
  mixInto(b, toStereo(room(m, 0.35, 0.4)), 0, 1);
  return b;
};

const sfxBlock = (): Stereo => {
  const b = metallic(0.4, 104, 520, [1, 1.42, 2.02], 0.35, 2.2);
  const m = mkMono(0.4);
  addOsc(m, 'sine', 0, 0.15, 140, 0.6, expDecay(0.5));
  mixInto(b, toStereo(m), 0, 1);
  return b;
};

const sfxImpactHeavy = (): Stereo => {
  const rand = rng(105);
  const m = mkMono(0.9);
  addOsc(m, 'sine', 0, 0.7, (u) => 120 * Math.pow(0.27, u) + 28, 0.9, expDecay(0.55));
  addNoise(m, rand, 0, 0.25, 0.7, expDecay(0.5));
  lowpass(m, 900);
  softClip(m, 1.8);
  return toStereo(room(m, 0.4, 0.5));
};

const footstep = (seed: number, weight: number): Stereo => {
  const rand = rng(seed);
  const m = mkMono(0.2);
  addNoise(m, rand, 0, 0.09, 0.5 * weight, adsr(0.08, 0.3, 0.4, 0.6));
  lowpass(m, 700 + rand() * 300);
  addOsc(m, 'sine', 0, 0.1, 95 + rand() * 25, 0.5 * weight, expDecay(0.45));
  return toStereo(m, (rand() - 0.5) * 0.3);
};

const sfxLanding = (): Stereo => {
  const rand = rng(107);
  const m = mkMono(0.55);
  addNoise(m, rand, 0, 0.2, 0.6, adsr(0.05, 0.4, 0.3, 0.6));
  lowpass(m, 800);
  addOsc(m, 'sine', 0, 0.3, (u) => 100 - 55 * u, 0.7, expDecay(0.5));
  // scattered dirt ticks
  for (let i = 0; i < 7; i++) {
    const tick = mkMono(0.2);
    const tr = rng(1070 + i);
    addNoise(tick, tr, 0, 0.02 + tr() * 0.02, 0.25, expDecay(0.4));
    bandpass(tick, 900 + tr() * 2200, 3);
    mixMono(m, tick, 0.12 + tr() * 0.3, 0.8);
  }
  return toStereo(room(m, 0.3, 0.35));
};

const sfxDebris = (): Stereo => {
  const rand = rng(108);
  const m = mkMono(0.8);
  for (let i = 0; i < 9; i++) {
    const tick = mkMono(0.15);
    addNoise(tick, rand, 0, 0.03 + rand() * 0.04, 0.5, expDecay(0.5));
    bandpass(tick, 700 + rand() * 2600, 4);
    addOsc(tick, 'sine', 0, 0.05, 300 + rand() * 900, 0.15, expDecay(0.4));
    mixMono(m, tick, 0.02 + rand() * 0.55, 0.5 + rand() * 0.5);
  }
  return toStereo(room(m, 0.3, 0.35));
};

const sfxPowerUp = (): Stereo => {
  const rand = rng(109);
  const m = mkMono(2.6);
  // rising shimmer bed
  addNoise(m, rand, 0.1, 2.3, 0.16, adsr(0.5, 0.1, 1, 0.15));
  bandpass(m, 2600, 1.1);
  // rising tone stack
  addOsc(m, 'saw', 0, 2.35, (u) => 110 * Math.pow(4, u), 0.14, adsr(0.45, 0.05, 0.9, 0.1));
  addOsc(m, 'saw', 0, 2.35, (u) => 110.9 * Math.pow(4, u), 0.14, adsr(0.45, 0.05, 0.9, 0.1));
  addOsc(m, 'sine', 0, 2.4, (u) => 220 * Math.pow(3.2, u), 0.1, adsr(0.4, 0.1, 0.8, 0.15));
  // end sparkle
  for (let i = 0; i < 10; i++) {
    addOsc(
      m,
      'sine',
      2.15 + rand() * 0.4,
      0.14,
      1500 + rand() * 2600,
      0.1,
      expDecay(0.6)
    );
  }
  lowpass(m, 6200);
  const wet = room(m, 0.35, 0.4);
  return toStereo(wet);
};

const sfxTitleImpact = (): Stereo => {
  const rand = rng(110);
  const m = mkMono(2.2);
  addOsc(m, 'sine', 0, 1.8, (u) => 68 * Math.pow(0.4, u), 0.95, expDecay(0.5));
  addNoise(m, rand, 0, 0.3, 0.5, expDecay(0.45));
  lowpass(m, 1400);
  // metallic clang layered on top
  const met = mkMono(1.6);
  const mr = rng(111);
  for (const p of [1, 1.61, 2.21, 3.02]) {
    addOsc(met, 'sine', 0, 1.5, 390 * p * (1 + (mr() - 0.5) * 0.01), 0.09, expDecay(0.9 * (0.7 + p * 0.1)));
  }
  mixMono(m, met, 0.015, 0.6);
  softClip(m, 1.5);
  return toStereo(room(m, 0.5, 0.6));
};

const sfxUi = (seed: number, f: number, dur: number): Stereo => {
  const m = mkMono(dur);
  addOsc(m, 'sine', 0, dur, f, 0.4, adsr(0.06, 0.2, 0.5, 0.5));
  addOsc(m, 'sine', 0, dur, f * 1.5, 0.14, adsr(0.06, 0.3, 0.3, 0.5));
  return toStereo(m);
};

// ---------- ambience ----------

const ambience = (): Stereo => {
  const dur = 24; // seconds, seamless loop
  const rand = rng(500);
  const m = mkMono(dur);

  // low rumble — brown-ish noise via heavy lowpass of white noise
  const rumble = mkMono(dur);
  addNoise(rumble, rand, 0, dur, 0.9);
  lowpass(rumble, 55, 3);
  for (let i = 0; i < m.length; i++) m[i] += rumble[i];

  // machine hum — 50Hz + harmonics with slow LFO swell
  for (const [f, g] of [
    [50, 0.16],
    [100, 0.07],
    [150, 0.035],
  ] as const) {
    const n = m.length;
    let phase = 0;
    for (let i = 0; i < n; i++) {
      const t = i / SR;
      const lfo = 0.75 + 0.25 * Math.sin(2 * Math.PI * 0.07 * t + f);
      phase += f / SR;
      m[i] += Math.sin(2 * Math.PI * phase) * g * lfo;
    }
  }

  // filtered air noise
  const air = mkMono(dur);
  addNoise(air, rand, 0, dur, 0.12);
  bandpass(air, 640, 0.7);
  lowpass(air, 1400);
  for (let i = 0; i < m.length; i++) m[i] += air[i];

  // occasional metallic resonance pings (kept inside the loop so it
  // wraps cleanly — last event's tail decays before the loop point)
  const events = [1.4, 5.8, 9.3, 13.7, 17.1, 20.9];
  for (const et of events) {
    const e = mkMono(1.4);
    const er = rng(500 + Math.round(et * 10));
    const f = 620 + er() * 900;
    addOsc(e, 'sine', 0, 1.3, f, 0.05, expDecay(0.55));
    addOsc(e, 'sine', 0, 1.3, f * 2.7, 0.02, expDecay(0.7));
    const wet = room(e, 0.7, 0.5);
    // place fully inside loop (mono — stereo width is added at the end)
    const at = Math.min(et, dur - 1.6);
    mixMono(m, wet, at, 1);
  }

  // seamless loop: crossfade final 0.5s into start
  const xfade = Math.round(0.5 * SR);
  const tail = m.slice(m.length - xfade);
  for (let i = 0; i < xfade; i++) {
    const k = i / xfade;
    m[i] = m[i] * (1 - k) + tail[i] * k * 0.5;
  }

  // stereo-ize with a short decorrelated copy
  const st = mkBuf(dur);
  const m2 = Float32Array.from(m);
  lowpass(m2, 320, 1);
  for (let i = 0; i < m.length; i++) {
    st.l[i] = m[i] * 0.9 + m2[i] * 0.1;
    st.r[i] = m2[i] * 0.9 + m[i] * 0.1;
  }
  return normalize(st, 0.5);
};

// ---------- run ----------

export const generateSfx = (root: string): void => {
  const out = (name: string, b: Stereo, generator: string, seed: number) => {
    const rel = `${SFX_DIR}/${name}.wav`;
    normalize(b);
    writeWav(b, `${root}/${rel}`);
    reg(`audio/generated/sfx/${name}.wav`, generator, seed, b.l.length / SR);
  };

  out('swing_light', sfxSwingLight(), 'whoosh:noise+beep-sweep', 101);
  out('swing_heavy', sfxSwingHeavy(), 'whoosh:noise+tri-sweep', 102);
  out('clash', sfxClash(), 'metallic-partials+noise', 103);
  out('block', sfxBlock(), 'metallic-partials+thump', 104);
  out('impact_heavy', sfxImpactHeavy(), 'sub-drop+noise+room', 105);
  out('footstep_a', footstep(1061, 1.0), 'noise-burst+thump', 1061);
  out('footstep_b', footstep(1062, 0.85), 'noise-burst+thump', 1062);
  out('landing', sfxLanding(), 'thump+scatter-ticks', 107);
  out('debris', sfxDebris(), 'seeded-tick-cluster', 108);
  out('powerup', sfxPowerUp(), 'rising-saws+shimmer', 109);
  out('title_impact', sfxTitleImpact(), 'boom+metal+room', 110);
  out('ui_select', sfxUi(112, 1750, 0.09), 'sine-blip', 112);
  out('ui_dialogue', sfxUi(113, 880, 0.12), 'soft-tick', 113);

  const amb = ambience();
  writeWav(amb, `${root}/${AMB_DIR}/ambience_loop.wav`);
  reg('audio/generated/ambience/ambience_loop.wav', 'rumble+hum+air+resonance', 500, amb.l.length / SR);
};

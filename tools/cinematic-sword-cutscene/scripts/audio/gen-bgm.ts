// Procedural 50s BGM — drone / bass pulse / percussion / riser / sub impact.
// Section plan mirrors the master timeline:
//   0-20   dark ambient drone
//   20-29  + tension pulse
//   29-39  combat rhythm
//   39-43  decay
//   43-47  riser / crescendo
//   47     stop
//   47-50  title drone
import {
  SR,
  type Stereo,
  addNoise,
  addOsc,
  adsr,
  bandpass,
  expDecay,
  highpass,
  lowpass,
  mkBuf,
  mkMono,
  mixMono,
  normalize,
  rng,
  softClip,
  writeWav,
} from './dsp.ts';
import type {GenInfo} from './gen-sfx.ts';

const DUR = 50;
const OUT = 'audio/generated/bgm/bgm.wav';

// Absolute-time gain windows: 1 inside [a,b] with edge ramps.
const win = (a: number, b: number, ramp = 0.4) => (t: number): number => {
  if (t < a || t > b) return 0;
  return Math.min(1, (t - a) / ramp, (b - t) / ramp);
};

// Oscillator whose gain follows an absolute-time window function.
const drone = (m: Float32Array, f: number, kind: 'saw' | 'sine' | 'tri', g: number,
  envT: (t: number) => number, detuneCents = 0): void => {
  const n = m.length;
  let phase = 0;
  const freq = f * Math.pow(2, detuneCents / 1200);
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    phase += freq / SR;
    const p = phase - Math.floor(phase);
    const s =
      kind === 'saw' ? 2 * p - 1 : kind === 'tri' ? (p < 0.5 ? 4 * p - 1 : 3 - 4 * p) : Math.sin(2 * Math.PI * p);
    m[i] += s * g * envT(t);
  }
};

const kick = (m: Float32Array, at: number, g = 0.8): void => {
  addOsc(m, 'sine', at, 0.22, (u) => 130 * Math.pow(0.3, u) + 38, g, expDecay(0.55));
  addNoise(m, rng(Math.round(at * 977) + 5), at, 0.015, 0.2, expDecay(0.4));
};

const hat = (m: Float32Array, at: number, g = 0.12): void => {
  const h = mkMono(0.06);
  addNoise(h, rng(Math.round(at * 1319) + 11), 0, 0.05, g * 4, expDecay(0.5));
  highpass(h, 6500);
  mixMono(m, h, at, 1);
};

const snare = (m: Float32Array, at: number, g = 0.4): void => {
  const s = mkMono(0.16);
  addNoise(s, rng(Math.round(at * 821) + 3), 0, 0.14, g, expDecay(0.5));
  bandpass(s, 1900, 0.9);
  addOsc(s, 'tri', 0, 0.08, 185, g * 0.5, expDecay(0.5));
  mixMono(m, s, at, 1);
};

const bassPulse = (m: Float32Array, at: number, g = 0.34): void => {
  const p = mkMono(0.3);
  addOsc(p, 'sine', 0, 0.28, 55, g * 2.4, adsr(0.03, 0.25, 0.25, 0.5));
  addOsc(p, 'saw', 0, 0.2, 110, g * 0.5, adsr(0.03, 0.3, 0.2, 0.5));
  lowpass(p, 320);
  mixMono(m, p, at, 1);
};

export const generateBgm = (root: string): GenInfo => {
  const seed = 777;
  const rand = rng(seed);
  const m = mkMono(DUR);

  // --- drone (A1/A2 minor-ish stack), always on under everything ---
  const droneWin = (t: number): number => {
    if (t >= 47) return 0;
    // dip during decay section then recover slightly into the riser
    const decay = t < 39 ? 1 : t < 43 ? 1 - (t - 39) / 4 * 0.55 : 0.45 + (t - 43) / 4 * 0.35;
    const fadeIn = Math.min(1, t / 2.5);
    return fadeIn * decay;
  };
  drone(m, 55, 'saw', 0.1, droneWin);
  drone(m, 55, 'saw', 0.1, droneWin, 7);
  drone(m, 110, 'tri', 0.05, droneWin, -5);
  drone(m, 164.8, 'saw', 0.028, droneWin, 4); // E3 — minor third colour
  // airy noise bed
  const bed = mkMono(DUR);
  addNoise(bed, rand, 0, DUR, 0.05);
  lowpass(bed, 420);
  mixMono(m, bed, 0, 1);
  // keep the drone dark
  lowpass(m, 900);

  // --- tension pulse 20-39 (runs under combat too) ---
  for (let t = 20; t < 39; t += 0.625) {
    const edge = Math.min(1, (t - 20) / 1.2, (39 - t) / 1.2);
    bassPulse(m, t, 0.34 * Math.max(0, edge));
  }

  // --- combat rhythm 29-39, 120bpm ---
  for (let t = 29; t < 39; t += 0.5) {
    const beat = Math.round((t - 29) / 0.5);
    kick(m, t, beat % 4 === 3 ? 0.6 : 0.85);
    if (beat % 2 === 1) snare(m, t, 0.4);
    hat(m, t + 0.25, 0.12);
    hat(m, t, 0.06);
  }
  // fill leading into the KAEL knockback
  for (let t = 34.4; t < 34.95; t += 0.125) hat(m, t, 0.1);

  // --- riser 43-47 ---
  const riser = mkMono(4.2);
  addNoise(riser, rand, 0, 4.2, 0.5, (u) => u * u);
  // swept bandpass upward
  const lo = Float32Array.from(riser);
  const hi = Float32Array.from(riser);
  bandpass(lo, 600, 1.2);
  bandpass(hi, 5200, 1.6);
  for (let i = 0; i < riser.length; i++) {
    const u = i / riser.length;
    riser[i] = lo[i] * (1 - u) + hi[i] * u;
  }
  mixMono(m, riser, 43, 1);
  // accelerating ticks
  let t = 43.2;
  let step = 0.4;
  while (t < 46.9) {
    hat(m, t, 0.14 * (0.5 + (t - 43) / 4));
    t += step;
    step = Math.max(0.11, step * 0.93);
  }
  drone(m, 110, 'saw', 0.05, win(43, 47, 3.9), 0);

  // --- hard stop at 47 ---
  const cut = Math.round(47 * SR);
  for (let i = cut; i < m.length; i++) m[i] = 0;
  // 6ms safety fade to avoid a click
  for (let i = 0; i < Math.round(0.006 * SR); i++) {
    const k = 1 - i / (0.006 * SR);
    m[cut - i] *= k;
  }

  // --- title drone 47.2-50 ---
  const td = mkMono(3);
  addOsc(td, 'sine', 0, 3, 55, 0.4, adsr(0.02, 0.1, 0.9, 0.35));
  addOsc(td, 'sine', 0, 3, 110.4, 0.16, adsr(0.02, 0.1, 0.85, 0.35));
  addOsc(td, 'sine', 0.25, 2.5, 880, 0.02, adsr(0.4, 0.2, 0.5, 0.5));
  lowpass(td, 1200);
  mixMono(m, td, 47.2, 1);

  softClip(m, 1.2);
  const st = mkBuf(DUR);
  // widen: short delay on R
  const delay = Math.round(0.008 * SR);
  for (let i = 0; i < m.length; i++) {
    st.l[i] = m[i];
    st.r[i] = i >= delay ? m[i - delay] * 0.92 : 0;
  }
  normalize(st, 0.55);
  writeWav(st, `${root}/assets/${OUT}`);
  return {file: OUT, generator: 'procedural-bgm', seed, durationSec: DUR};
};

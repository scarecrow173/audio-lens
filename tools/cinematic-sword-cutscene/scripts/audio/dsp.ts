// Deterministic procedural-audio DSP toolkit.
// All randomness flows through the seeded RNG — same seed => same WAV bytes.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

export const SR = 44100;

export type Stereo = { l: Float32Array; r: Float32Array };

export const mkBuf = (seconds: number): Stereo => ({
  l: new Float32Array(Math.ceil(seconds * SR)),
  r: new Float32Array(Math.ceil(seconds * SR)),
});

export const mkMono = (seconds: number): Float32Array => new Float32Array(Math.ceil(seconds * SR));

// mulberry32 — identical to src/utils/seededRandom's generator.
export const rng = (seed: number): (() => number) => {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

// ---------- oscillators ----------

export type OscKind = "sine" | "tri" | "saw" | "square";

const oscSample = (kind: OscKind, phase: number): number => {
  const p = phase - Math.floor(phase);
  switch (kind) {
    case "sine":
      return Math.sin(2 * Math.PI * p);
    case "tri":
      return p < 0.5 ? 4 * p - 1 : 3 - 4 * p;
    case "saw":
      return 2 * p - 1;
    case "square":
      return p < 0.5 ? 1 : -1;
  }
};

// Add an oscillator into buf[start..start+dur]. freq may be a function of
// normalized time (0..1) for sweeps.
export const addOsc = (
  buf: Float32Array,
  kind: OscKind,
  atSec: number,
  durSec: number,
  freq: number | ((u: number) => number),
  gain: number,
  env?: (u: number) => number,
  detune = 0,
): void => {
  const start = Math.round(atSec * SR);
  const n = Math.round(durSec * SR);
  let phase = 0;
  for (let i = 0; i < n && start + i < buf.length; i++) {
    const u = i / n;
    const f = (typeof freq === "function" ? freq(u) : freq) + detune;
    phase += f / SR;
    const e = env ? env(u) : 1;
    buf[start + i] += oscSample(kind, phase) * gain * e;
  }
};

export const addNoise = (
  buf: Float32Array,
  rand: () => number,
  atSec: number,
  durSec: number,
  gain: number,
  env?: (u: number) => number,
): void => {
  const start = Math.round(atSec * SR);
  const n = Math.round(durSec * SR);
  for (let i = 0; i < n && start + i < buf.length; i++) {
    const u = i / n;
    const e = env ? env(u) : 1;
    buf[start + i] += (rand() * 2 - 1) * gain * e;
  }
};

// ---------- envelopes ----------

export const expDecay = (decay: number) => (u: number) => Math.exp(-u * decay * 6.9); // ~-60dB at u=1*decay scaling

export const adsr =
  (a: number, d: number, s: number, r: number) =>
  (u: number): number => {
    if (u < a) return u / a;
    if (u < a + d) return 1 - (1 - s) * ((u - a) / d);
    if (u > 1 - r) return s * Math.max(0, (1 - u) / r);
    return s;
  };

export const gate =
  (attack = 0.01, release = 0.05) =>
  (u: number): number =>
    Math.min(1, u / attack, (1 - u) / release);

// ---------- filters (stateful, process in place or return new) ----------

export const lowpass = (buf: Float32Array, cutoff: number, passes = 1): Float32Array => {
  const a = 1 - Math.exp((-2 * Math.PI * cutoff) / SR);
  for (let p = 0; p < passes; p++) {
    let y = 0;
    for (let i = 0; i < buf.length; i++) {
      y += a * (buf[i] - y);
      buf[i] = y;
    }
  }
  return buf;
};

export const highpass = (buf: Float32Array, cutoff: number): Float32Array => {
  const a = Math.exp((-2 * Math.PI * cutoff) / SR);
  let y = 0;
  let xPrev = 0;
  for (let i = 0; i < buf.length; i++) {
    const x = buf[i];
    y = a * (y + x - xPrev);
    xPrev = x;
    buf[i] = y;
  }
  return buf;
};

// Simple resonant band-pass (biquad, constant skirt gain).
export const bandpass = (buf: Float32Array, freq: number, q: number): Float32Array => {
  const w = (2 * Math.PI * freq) / SR;
  const alpha = Math.sin(w) / (2 * q);
  const b0 = alpha;
  const b1 = 0;
  const b2 = -alpha;
  const a0 = 1 + alpha;
  const a1 = -2 * Math.cos(w);
  const a2 = 1 - alpha;
  let x1 = 0;
  let x2 = 0;
  let y1 = 0;
  let y2 = 0;
  for (let i = 0; i < buf.length; i++) {
    const x0 = buf[i];
    const y0 = (b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
    x2 = x1;
    x1 = x0;
    y2 = y1;
    y1 = y0;
    buf[i] = y0;
  }
  return buf;
};

export const lowShelf = (buf: Float32Array, freq: number, gainDb: number): Float32Array => {
  const a = 10 ** (gainDb / 40);
  const w = (2 * Math.PI * freq) / SR;
  const cw = Math.cos(w);
  const alpha = (Math.sin(w) / 2) * Math.sqrt(2);
  const b0 = a * (a + 1 - (a - 1) * cw + 2 * Math.sqrt(a) * alpha);
  const b1 = 2 * a * (a - 1 - (a + 1) * cw);
  const b2 = a * (a + 1 - (a - 1) * cw - 2 * Math.sqrt(a) * alpha);
  const a0 = a + 1 + (a - 1) * cw + 2 * Math.sqrt(a) * alpha;
  const a1 = -2 * (a - 1 + (a + 1) * cw);
  const a2 = a + 1 + (a - 1) * cw - 2 * Math.sqrt(a) * alpha;
  let x1 = 0;
  let x2 = 0;
  let y1 = 0;
  let y2 = 0;
  for (let i = 0; i < buf.length; i++) {
    const x0 = buf[i];
    const y0 = (b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
    x2 = x1;
    x1 = x0;
    y2 = y1;
    y1 = y0;
    buf[i] = y0;
  }
  return buf;
};

export const softClip = (buf: Float32Array, drive: number): Float32Array => {
  for (let i = 0; i < buf.length; i++) buf[i] = Math.tanh(buf[i] * drive);
  return buf;
};

// Biquad peaking EQ (RBJ cookbook), in place.
export const peakingEq = (
  buf: Float32Array,
  freq: number,
  q: number,
  gainDb: number,
): Float32Array => {
  const a = 10 ** (gainDb / 40);
  const w = (2 * Math.PI * freq) / SR;
  const alpha = Math.sin(w) / (2 * q);
  const b0 = 1 + alpha * a;
  const b1 = -2 * Math.cos(w);
  const b2 = 1 - alpha * a;
  const a0 = 1 + alpha / a;
  const a1 = -2 * Math.cos(w);
  const a2 = 1 - alpha / a;
  let x1 = 0;
  let x2 = 0;
  let y1 = 0;
  let y2 = 0;
  for (let i = 0; i < buf.length; i++) {
    const x0 = buf[i];
    const y0 = (b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
    x2 = x1;
    x1 = x0;
    y2 = y1;
    y1 = y0;
    buf[i] = y0;
  }
  return buf;
};

// Downward compressor with RMS-ish envelope follower, in place.
export const compressor = (
  buf: Float32Array,
  thresholdDb: number,
  ratio: number,
  attackMs = 8,
  releaseMs = 140,
  makeupDb = 0,
): Float32Array => {
  const thr = 10 ** (thresholdDb / 20);
  const att = Math.exp(-1 / ((attackMs / 1000) * SR));
  const rel = Math.exp(-1 / ((releaseMs / 1000) * SR));
  const makeup = 10 ** (makeupDb / 20);
  let env = 0;
  for (let i = 0; i < buf.length; i++) {
    const x = Math.abs(buf[i]);
    env = x > env ? att * env + (1 - att) * x : rel * env + (1 - rel) * x;
    let g = 1;
    if (env > thr) g = (thr / env) ** (1 - 1 / ratio);
    buf[i] *= g * makeup;
  }
  return buf;
};

// Pitch shift by resampling (changes pitch AND duration — a "tape"
// speed change, which also shifts formants, useful for voice contrast).
// Catmull-Rom cubic interpolation.
export const resampleRate = (src: Float32Array, ratio: number): Float32Array => {
  const out = new Float32Array(Math.round(src.length / ratio));
  for (let i = 0; i < out.length; i++) {
    const pos = i * ratio;
    const i0 = Math.floor(pos);
    const f = pos - i0;
    const g = (k: number) => src[Math.max(0, Math.min(src.length - 1, i0 + k))];
    const p0 = g(-1);
    const p1 = g(0);
    const p2 = g(1);
    const p3 = g(2);
    out[i] =
      p1 +
      0.5 * f * (p2 - p0 + f * (2 * p0 - 5 * p1 + 4 * p2 - p3 + f * (3 * (p1 - p2) + p3 - p0)));
  }
  return out;
};

// Trim silence below threshold from both ends, keeping edge padding.
export const trimSilence = (buf: Float32Array, thresholdDb = -45, padSec = 0.08): Float32Array => {
  const thr = 10 ** (thresholdDb / 20);
  let s = 0;
  let e = buf.length - 1;
  while (s < e && Math.abs(buf[s]) < thr) s++;
  while (e > s && Math.abs(buf[e]) < thr) e--;
  const pad = Math.round(padSec * SR);
  return buf.slice(Math.max(0, s - pad), Math.min(buf.length, e + pad));
};

// Feedback delay network "room" — 4 detuned combs + damping, cheap reverb.
export const room = (
  src: Float32Array,
  mixAmt: number,
  timeSec: number,
  damping = 0.4,
): Float32Array => {
  const out = new Float32Array(src.length);
  const combs = [0.0297, 0.0371, 0.0411, 0.0437].map((d) => ({
    size: Math.round(d * timeSec * SR * 30),
    buf: new Float32Array(Math.round(d * timeSec * SR * 30) + 8),
    idx: 0,
    filt: 0,
  }));
  for (let i = 0; i < src.length; i++) {
    const x = src[i];
    let wet = 0;
    for (const c of combs) {
      const y = c.buf[c.idx];
      c.filt = y * (1 - damping) + c.filt * damping;
      c.buf[c.idx] = x + c.filt * 0.78;
      wet += y;
      c.idx = (c.idx + 1) % c.size;
    }
    out[i] = x * (1 - mixAmt) + (wet / combs.length) * mixAmt;
  }
  return out;
};

// ---------- mixing / mastering ----------

export const toStereo = (src: Float32Array, pan = 0): Stereo => {
  const out = mkBuf(src.length / SR);
  const lg = Math.cos(((pan + 1) * Math.PI) / 4);
  const rg = Math.sin(((pan + 1) * Math.PI) / 4);
  for (let i = 0; i < src.length; i++) {
    out.l[i] += src[i] * lg;
    out.r[i] += src[i] * rg;
  }
  return out;
};

export const mixMono = (dst: Float32Array, src: Float32Array, atSec: number, gain = 1): void => {
  const start = Math.round(atSec * SR);
  const n = Math.min(src.length, dst.length - start);
  for (let i = 0; i < n; i++) dst[start + i] += src[i] * gain;
};

export const mixInto = (dst: Stereo, src: Stereo, atSec: number, gain = 1): void => {
  const start = Math.round(atSec * SR);
  const n = Math.min(src.l.length, dst.l.length - start);
  for (let i = 0; i < n; i++) {
    dst.l[start + i] += src.l[i] * gain;
    dst.r[start + i] += src.r[i] * gain;
  }
};

export const normalize = (buf: Stereo, peak = 0.89): Stereo => {
  let max = 0;
  for (let i = 0; i < buf.l.length; i++) {
    max = Math.max(max, Math.abs(buf.l[i]), Math.abs(buf.r[i]));
  }
  if (max > 0) {
    const g = peak / max;
    for (let i = 0; i < buf.l.length; i++) {
      buf.l[i] *= g;
      buf.r[i] *= g;
    }
  }
  return buf;
};

// ---------- WAV I/O ----------

// Read a WAV (PCM16 or IEEE float32, mono or stereo) into Stereo.
export const readWav = (path: string): { buf: Stereo; sr: number } => {
  const b = readFileSync(path);
  const fmt = b.readUInt16LE(20);
  const channels = b.readUInt16LE(22);
  const sr = b.readUInt32LE(24);
  const bits = b.readUInt16LE(34);
  // find the data chunk (skip possible extra chunks)
  let off = 12;
  let dataOff = 44;
  let dataSize = b.length - 44;
  while (off + 8 <= b.length) {
    const id = b.toString("ascii", off, off + 4);
    const size = b.readUInt32LE(off + 4);
    if (id === "data") {
      dataOff = off + 8;
      dataSize = size;
      break;
    }
    off += 8 + size;
  }
  const bytesPerSample = bits / 8;
  const n = Math.floor(dataSize / bytesPerSample / channels);
  const out = mkBuf(n / sr);
  const readSample = (i: number, ch: number): number => {
    const off = dataOff + (i * channels + ch) * bytesPerSample;
    if (fmt === 3) return b.readFloatLE(off); // IEEE float
    return b.readInt16LE(off) / 32768; // PCM16
  };
  for (let i = 0; i < n; i++) {
    const l = readSample(i, 0);
    const r = channels > 1 ? readSample(i, 1) : l;
    out.l[i] = l;
    out.r[i] = r;
  }
  return { buf: out, sr };
};

export const writeWav = (buf: Stereo, path: string): void => {
  const { l, r } = buf;
  const dataSize = l.length * 2 * 2;
  const header = new ArrayBuffer(44);
  const v = new DataView(header);
  const ws = (o: number, s: string) => {
    for (let i = 0; i < s.length; i++) v.setUint8(o + i, s.charCodeAt(i));
  };
  ws(0, "RIFF");
  v.setUint32(4, 36 + dataSize, true);
  ws(8, "WAVE");
  ws(12, "fmt ");
  v.setUint32(16, 16, true);
  v.setUint16(20, 1, true);
  v.setUint16(22, 2, true);
  v.setUint32(24, SR, true);
  v.setUint32(28, SR * 4, true);
  v.setUint16(32, 4, true);
  v.setUint16(34, 16, true);
  ws(36, "data");
  v.setUint32(40, dataSize, true);
  const pcm = new Int16Array(l.length * 2);
  for (let i = 0; i < l.length; i++) {
    pcm[i * 2] = Math.max(-1, Math.min(1, l[i])) * 32767;
    pcm[i * 2 + 1] = Math.max(-1, Math.min(1, r[i])) * 32767;
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, Buffer.concat([Buffer.from(header), Buffer.from(pcm.buffer)]));
};

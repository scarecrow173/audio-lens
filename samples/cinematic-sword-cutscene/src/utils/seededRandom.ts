// Deterministic seeded PRNG (mulberry32). Same seed -> same sequence.
export type Rng = () => number;

export const createRng = (seed: number): Rng => {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

// Stateless hash-based random: deterministic per (seed, index).
export const hashRandom = (seed: number, index: number): number => {
  let h = (seed ^ Math.imul(index + 1, 0x9e3779b1)) >>> 0;
  h ^= h >>> 15;
  h = Math.imul(h, 0x85ebca6b) >>> 0;
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35) >>> 0;
  h ^= h >>> 16;
  return h / 4294967296;
};

// Smooth deterministic 1D noise for camera shake etc.
export const noise1D = (t: number, seed: number): number => {
  const i = Math.floor(t);
  const f = t - i;
  const u = f * f * (3 - 2 * f);
  const a = hashRandom(seed, i) * 2 - 1;
  const b = hashRandom(seed, i + 1) * 2 - 1;
  return a + (b - a) * u;
};

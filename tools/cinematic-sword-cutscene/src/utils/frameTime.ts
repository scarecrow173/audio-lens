import { useVideoConfig } from "remotion";
import { useRemappedFrame } from "../bugs/RenderFrameContext";

export { useRemappedFrame };

// All gameplay state derives from the *remapped* source frame so that
// time-remap bugs (freeze, dropped frames, ...) freeze every subsystem
// consistently and remain bit-deterministic.
export const useGameFrame = (): number => useRemappedFrame();

export const useGameTime = (): number => {
  const { fps } = useVideoConfig();
  return useRemappedFrame() / fps;
};

export const clamp = (v: number, min = 0, max = 1): number => Math.min(max, Math.max(min, v));

export const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

export const smoothstep = (t: number): number => t * t * (3 - 2 * t);

export const easeInOut = (t: number): number => smoothstep(clamp(t));

export const easeOut = (t: number): number => 1 - (1 - clamp(t)) ** 3;

export type Key<T> = { t: number; v: T };

// Piecewise-linear keyframe evaluation with per-segment easing.
export const evalKeys = (
  keys: readonly Key<number>[],
  time: number,
  ease: (t: number) => number = easeInOut,
): number => {
  if (keys.length === 0) return 0;
  if (time <= keys[0].t) return keys[0].v;
  const last = keys[keys.length - 1];
  if (time >= last.t) return last.v;
  for (let i = 0; i < keys.length - 1; i++) {
    const a = keys[i];
    const b = keys[i + 1];
    if (time >= a.t && time < b.t) {
      return lerp(a.v, b.v, ease((time - a.t) / (b.t - a.t)));
    }
  }
  return last.v;
};

export const evalKeys3 = (
  keys: readonly Key<readonly [number, number, number]>[],
  time: number,
  ease: (t: number) => number = easeInOut,
): [number, number, number] => {
  if (keys.length === 0) return [0, 0, 0];
  if (time <= keys[0].t) return [...keys[0].v] as [number, number, number];
  const last = keys[keys.length - 1];
  if (time >= last.t) return [...last.v] as [number, number, number];
  for (let i = 0; i < keys.length - 1; i++) {
    const a = keys[i];
    const b = keys[i + 1];
    if (time >= a.t && time < b.t) {
      const k = ease((time - a.t) / (b.t - a.t));
      return [lerp(a.v[0], b.v[0], k), lerp(a.v[1], b.v[1], k), lerp(a.v[2], b.v[2], k)];
    }
  }
  return [...last.v] as [number, number, number];
};

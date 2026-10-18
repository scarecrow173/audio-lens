import type { ImpactEvent } from "../../data/shots";

// Impact helpers evaluated on master-timeline time.

// Flash intensity 0..1 at time t (hard spike, ~5 frames decay).
export const impactFlashAt = (
  impacts: readonly ImpactEvent[],
  t: number,
): { intensity: number; pos: readonly [number, number, number] | null } => {
  let best = 0;
  let pos: readonly [number, number, number] | null = null;
  for (const imp of impacts) {
    const dt = t - imp.t;
    if (dt < 0 || dt > 0.15) continue;
    const v = imp.power * (1 - dt / 0.15);
    if (v > best) {
      best = v;
      pos = imp.pos;
    }
  }
  return { intensity: best, pos };
};

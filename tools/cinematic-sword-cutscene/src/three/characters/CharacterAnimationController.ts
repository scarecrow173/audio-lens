import { clipDuration } from "../../data/kaykitManifest";
import type { AnimSeg, CharacterTrack } from "../../data/shots";
import { clamp, evalKeys, smoothstep } from "../../utils/frameTime";

export type ActiveClip = { clip: string; time: number; weight: number };

const HOLD_EPS = 1e-3;

// Resolve which clips are active at time `t`, with what local clip time and
// crossfade weight. Pure function of `t` -> seekable & deterministic.
export const activeClipsAt = (anims: readonly AnimSeg[], t: number): ActiveClip[] => {
  const out: ActiveClip[] = [];
  for (let i = 0; i < anims.length; i++) {
    const seg = anims[i];
    const next = anims[i + 1];
    const dur = clipDuration(seg.clip) / (seg.timeScale ?? 1);
    const segEnd = seg.end ?? seg.start + dur;
    const outBlend = next?.blend ?? 0.12;
    const effectiveEnd = segEnd + outBlend;
    if (t < seg.start || t >= effectiveEnd) continue;

    const inBlend = seg.blend ?? 0.12;
    const wIn = inBlend <= 0 ? 1 : smoothstep(clamp((t - seg.start) / inBlend));
    const wOut = t <= segEnd ? 1 : 1 - smoothstep(clamp((t - segEnd) / outBlend));
    const weight = wIn * wOut;
    if (weight <= 0) continue;

    const local = (t - seg.start) * (seg.timeScale ?? 1);
    const clipDur = clipDuration(seg.clip);
    const clipT = seg.loop
      ? clipDur > 0
        ? local % clipDur
        : 0
      : Math.min(local, clipDur - HOLD_EPS);
    out.push({ clip: seg.clip, time: Math.max(0, clipT), weight });
  }

  if (out.length === 0 && anims.length > 0) {
    // Gap in the track: hold the nearest earlier segment's final pose.
    let fallback = anims[0];
    for (const seg of anims) if (seg.start <= t) fallback = seg;
    const clipDur = clipDuration(fallback.clip);
    const endT = fallback.end ?? fallback.start + clipDur;
    const local = Math.min(endT - fallback.start, clipDur) * (fallback.timeScale ?? 1);
    out.push({
      clip: fallback.clip,
      time: Math.max(0, Math.min(local / (fallback.timeScale ?? 1), clipDur - HOLD_EPS)),
      weight: 1,
    });
  }
  return out;
};

export const positionAt = (
  track: CharacterTrack,
  t: number,
): { x: number; z: number; yaw: number } => {
  const xs = track.pos.map((k) => ({ t: k.t, v: k.v[0] }));
  const zs = track.pos.map((k) => ({ t: k.t, v: k.v[1] }));
  return {
    x: evalKeys(xs, t),
    z: evalKeys(zs, t),
    yaw: evalKeys(track.yaw, t),
  };
};

const ATTACK_RE = /Attack|Stab|Chop|Slice|Spin/;

export const isAttackingAt = (anims: readonly AnimSeg[], t: number): boolean =>
  activeClipsAt(anims, t).some((c) => c.weight > 0.35 && ATTACK_RE.test(c.clip));

export const activeClipNames = (anims: readonly AnimSeg[], t: number): string =>
  activeClipsAt(anims, t)
    .map((c) => `${c.clip}@${c.time.toFixed(2)} w${c.weight.toFixed(2)}`)
    .join(" | ") || "-";

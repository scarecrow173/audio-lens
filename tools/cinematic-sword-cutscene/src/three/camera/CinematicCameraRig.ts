import type { ImpactEvent, ShotDef } from "../../data/shots";
import { evalKeys3 } from "../../utils/frameTime";
import { noise1D } from "../../utils/seededRandom";

export type CameraPose = {
  pos: [number, number, number];
  look: [number, number, number];
  fov: number;
};

const SHAKE_SEED = 511;

// Camera shake envelope driven by impact events (deterministic noise).
const shakeAt = (t: number, impacts: readonly ImpactEvent[]): [number, number] => {
  let sx = 0;
  let sy = 0;
  for (const imp of impacts) {
    const dt = t - imp.t;
    if (dt < 0 || dt > 0.45) continue;
    const amp = imp.power * 0.06 * (1 - dt / 0.45);
    sx += amp * noise1D(t * 43, SHAKE_SEED);
    sy += amp * noise1D(t * 47, SHAKE_SEED + 7);
  }
  return [sx, sy];
};

// Evaluate the shot camera at time t. Shots are hard-cut: the pose inside a
// shot interpolates only between that shot's own keys.
export const cameraAt = (shot: ShotDef, t: number, impacts: readonly ImpactEvent[]): CameraPose => {
  const pos = evalKeys3(shot.camera.pos, t);
  const look = evalKeys3(shot.camera.look, t);
  const [sx, sy] = shakeAt(t, impacts);
  return {
    pos: [pos[0] + sx, pos[1] + sy, pos[2]],
    look: [look[0] + sx * 0.4, look[1] + sy * 0.4, look[2]],
    fov: shot.camera.fov ?? 42,
  };
};

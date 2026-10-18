// Shared timeline types. All times are SECONDS on the master timeline;
// frames are derived at 30fps.

export type Vec3 = readonly [number, number, number];
export type Vec2 = readonly [number, number];

export type Timed<T> = { t: number; v: T };

// Character animation segment. The controller evaluates which segments
// cover time `t` and assigns each a crossfade weight.
export type AnimSeg = {
  clip: string;
  start: number;
  // Absent = play clip once for its authored duration.
  end?: number;
  // Loop the clip across [start,end].
  loop?: boolean;
  // Crossfade-in duration (default 0.12s).
  blend?: number;
  timeScale?: number;
};

// Waypoints on the ground plane; eased between keys.
export type PosKey = Timed<Vec2>;
export type YawKey = Timed<number>;

export type CharacterTrack = {
  anims: AnimSeg[];
  pos: PosKey[];
  yaw: YawKey[];
};

export type CameraTrack = {
  pos: Timed<Vec3>[];
  look: Timed<Vec3>[];
  fov?: number;
};

export type ShotDef = {
  id: string;
  start: number;
  end: number;
  camera: CameraTrack;
  dialogue?: { speaker: "KAEL" | "VAREN"; text: string };
  // Overlay covering the whole frame (used for the cut-to-black ending).
  blackout?: boolean;
  title?: { line1: string; line2: string; from: number; to: number };
};

export type ImpactEvent = {
  t: number;
  pos: Vec3;
  // Seconds of camera shake + flash strength.
  power: number;
};

export type Timeline = {
  fps: number;
  durationSec: number;
  shots: ShotDef[];
  kael: CharacterTrack;
  varen: CharacterTrack;
  impacts: ImpactEvent[];
  debrisBursts: { t: number; pos: Vec3 }[];
  dustBursts: { t: number; pos: Vec3 }[];
  warningPulses: { t: number; dur: number }[];
  powerUp: { start: number; end: number } | null;
  combatHudFrom: number;
  hp: { kael: Timed<number>[]; varen: Timed<number>[] };
};

export const shotAt = (tl: Timeline, t: number): ShotDef | undefined =>
  tl.shots.find((s) => t >= s.start && t < s.end);

export const shotIndexAt = (tl: Timeline, t: number): number =>
  tl.shots.findIndex((s) => t >= s.start && t < s.end);

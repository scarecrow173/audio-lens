import { ANIM } from "./animations";
import type { Timeline } from "./shots";

// Master timeline: the single source of truth for the NORMAL render.
// Bug renders replay this exact timeline through a frame remap.

const KAEL_HOME: readonly [number, number] = [-3.2, 1.5];
const VAREN_HOME: readonly [number, number] = [3.2, -1.5];
// Characters face each other along +x/-x (model forward = +Z).
const KAEL_YAW = Math.PI / 2;
const VAREN_YAW = -Math.PI / 2;

export const MASTER_TIMELINE: Timeline = {
  fps: 30,
  durationSec: 50,

  shots: [
    {
      id: "S01",
      start: 0,
      end: 4.5,
      camera: {
        pos: [
          { t: 0, v: [-4.6, 3.4, 8.2] },
          { t: 4.5, v: [-3.6, 2.9, 7.0] },
        ],
        look: [
          { t: 0, v: [0.6, 1.0, -0.6] },
          { t: 4.5, v: [0.4, 1.0, -0.4] },
        ],
        fov: 46,
      },
    },
    {
      id: "S02",
      start: 4.5,
      end: 9.0,
      camera: {
        pos: [
          { t: 4.5, v: [-4.7, 1.8, 3.5] },
          { t: 9.0, v: [-4.4, 1.7, 3.2] },
        ],
        look: [
          { t: 4.5, v: [3.0, 1.3, -1.4] },
          { t: 9.0, v: [3.0, 1.25, -1.4] },
        ],
      },
      dialogue: { speaker: "KAEL", text: "……まだ終わっていない。" },
    },
    {
      id: "S03",
      start: 9.0,
      end: 13.5,
      camera: {
        pos: [
          { t: 9.0, v: [4.7, 2.0, -3.1] },
          { t: 13.5, v: [4.4, 1.9, -2.8] },
        ],
        look: [
          { t: 9.0, v: [-3.1, 1.2, 1.4] },
          { t: 13.5, v: [-3.1, 1.15, 1.4] },
        ],
      },
      dialogue: { speaker: "VAREN", text: "終わっていない？ いや、もう決して戻れない。" },
    },
    {
      id: "S04",
      start: 13.5,
      end: 17.0,
      camera: {
        pos: [
          { t: 13.5, v: [-1.2, 1.4, 3.2] },
          { t: 17.0, v: [-1.5, 1.35, 3.0] },
        ],
        look: [
          { t: 13.5, v: [-3.1, 1.35, 1.55] },
          { t: 17.0, v: [-3.1, 1.3, 1.55] },
        ],
      },
      dialogue: { speaker: "KAEL", text: "お前をここで止める。" },
    },
    {
      id: "S05",
      start: 17.0,
      end: 20.5,
      camera: {
        pos: [
          { t: 17.0, v: [1.4, 1.7, -0.2] },
          { t: 20.5, v: [1.7, 1.6, -0.4] },
        ],
        look: [
          { t: 17.0, v: [3.2, 1.5, -1.5] },
          { t: 20.5, v: [3.2, 1.45, -1.5] },
        ],
      },
      dialogue: { speaker: "VAREN", text: "なら見せてみろ。" },
    },
    {
      id: "S06",
      start: 20.5,
      end: 22.5,
      camera: {
        pos: [
          { t: 20.5, v: [-2.0, 1.15, 4.4] },
          { t: 22.5, v: [-2.3, 1.1, 4.1] },
        ],
        look: [
          { t: 20.5, v: [-3.2, 1.0, 1.5] },
          { t: 22.5, v: [-3.2, 1.05, 1.5] },
        ],
      },
    },
    {
      id: "S07",
      start: 22.5,
      end: 24.5,
      camera: {
        pos: [
          { t: 22.5, v: [4.8, 1.3, -4.1] },
          { t: 24.5, v: [4.5, 1.25, -3.7] },
        ],
        look: [
          { t: 22.5, v: [3.2, 1.2, -1.5] },
          { t: 24.5, v: [3.2, 1.15, -1.5] },
        ],
      },
    },
    {
      id: "S08",
      start: 24.5,
      end: 28.5,
      camera: {
        pos: [
          { t: 24.5, v: [-0.2, 1.8, 6.4] },
          { t: 28.5, v: [0.3, 1.65, 5.8] },
        ],
        look: [
          { t: 24.5, v: [0.0, 1.0, 0.3] },
          { t: 28.5, v: [0.0, 0.95, 0.4] },
        ],
        fov: 46,
      },
    },
    {
      id: "S09",
      start: 28.5,
      end: 29.4,
      camera: {
        pos: [{ t: 28.5, v: [-1.7, 1.3, 2.9] }],
        look: [{ t: 28.5, v: [-3.1, 1.3, 1.4] }],
      },
    },
    {
      id: "S10",
      start: 29.4,
      end: 31.2,
      camera: {
        pos: [
          { t: 29.4, v: [-2.4, 1.5, 4.7] },
          { t: 31.2, v: [-0.4, 1.4, 4.2] },
        ],
        look: [
          { t: 29.4, v: [-2.2, 1.1, 0.8] },
          { t: 31.2, v: [-0.2, 1.1, 0.2] },
        ],
      },
    },
    {
      id: "S11",
      start: 31.2,
      end: 32.3,
      camera: {
        pos: [
          { t: 31.2, v: [0.2, 1.7, 4.6] },
          { t: 32.3, v: [0.35, 1.6, 4.4] },
        ],
        look: [
          { t: 31.2, v: [0.1, 1.15, 0.2] },
          { t: 32.3, v: [0.1, 1.1, 0.2] },
        ],
      },
    },
    {
      id: "S12",
      start: 32.3,
      end: 34.0,
      camera: {
        pos: [
          { t: 32.3, v: [0.9, 1.6, 4.6] },
          { t: 34.0, v: [1.7, 1.45, 3.7] },
        ],
        look: [
          { t: 32.3, v: [0.15, 1.05, 0.2] },
          { t: 34.0, v: [0.3, 1.05, 0.1] },
        ],
      },
    },
    {
      id: "S13",
      start: 34.0,
      end: 35.8,
      camera: {
        pos: [
          { t: 34.0, v: [-0.8, 1.7, 3.5] },
          { t: 35.8, v: [-1.0, 1.6, 3.3] },
        ],
        look: [
          { t: 34.0, v: [0.9, 1.15, 0.1] },
          { t: 35.8, v: [0.6, 1.05, 0.2] },
        ],
      },
    },
    {
      id: "S14",
      start: 35.8,
      end: 37.6,
      camera: {
        pos: [
          { t: 35.8, v: [0.6, 1.9, 4.9] },
          { t: 37.6, v: [0.4, 1.7, 4.7] },
        ],
        look: [
          { t: 35.8, v: [-0.8, 1.0, 0.3] },
          { t: 37.6, v: [-1.8, 0.8, 0.4] },
        ],
      },
    },
    {
      id: "S15",
      start: 37.6,
      end: 38.8,
      camera: {
        pos: [{ t: 37.6, v: [-1.4, 0.9, 3.4] }],
        look: [{ t: 37.6, v: [-2.7, 0.6, 0.45] }],
      },
    },
    {
      id: "S16",
      start: 38.8,
      end: 41.5,
      camera: {
        pos: [
          { t: 38.8, v: [-2.4, 1.8, 4.6] },
          { t: 40.1, v: [-4.4, 1.7, 1.2] },
          { t: 41.5, v: [-3.6, 1.6, -2.4] },
        ],
        look: [
          { t: 38.8, v: [-0.8, 1.0, 0.1] },
          { t: 41.5, v: [-0.9, 1.0, 0.2] },
        ],
      },
    },
    {
      id: "S17",
      start: 41.5,
      end: 43.3,
      camera: {
        pos: [
          { t: 41.5, v: [-1.2, 1.45, 4.0] },
          { t: 43.3, v: [-1.4, 1.4, 3.8] },
        ],
        look: [
          { t: 41.5, v: [0.6, 1.2, 0.2] },
          { t: 43.3, v: [-0.2, 1.15, 0.25] },
        ],
      },
      dialogue: { speaker: "VAREN", text: "その程度か。" },
    },
    {
      id: "S18",
      start: 43.3,
      end: 46.0,
      camera: {
        pos: [
          { t: 43.3, v: [-1.2, 1.25, 3.4] },
          { t: 46.0, v: [-1.6, 1.15, 3.0] },
        ],
        look: [
          { t: 43.3, v: [-2.7, 1.15, 0.45] },
          { t: 46.0, v: [-2.7, 1.2, 0.45] },
        ],
      },
    },
    {
      id: "S19",
      start: 46.0,
      end: 47.2,
      camera: {
        pos: [
          { t: 46.0, v: [0.8, 1.35, 3.3] },
          { t: 47.2, v: [-0.3, 1.2, 2.6] },
        ],
        look: [
          { t: 46.0, v: [-1.2, 1.1, 0.3] },
          { t: 47.2, v: [-0.3, 1.15, 0.25] },
        ],
      },
    },
    {
      id: "S20",
      start: 47.2,
      end: 50.0,
      camera: { pos: [{ t: 47.2, v: [0, 1.4, 5] }], look: [{ t: 47.2, v: [0, 1, 0] }] },
      blackout: true,
      title: { line1: "THE LAST SIGNAL", line2: "CHAPTER 03", from: 47.7, to: 49.7 },
    },
  ],

  kael: {
    pos: [
      { t: 0, v: KAEL_HOME },
      { t: 29.4, v: KAEL_HOME },
      { t: 30.75, v: [-0.7, 0.2] },
      { t: 34.95, v: [-0.6, 0.25] },
      { t: 35.7, v: [-2.7, 0.45] },
      { t: 46.0, v: [-2.7, 0.45] },
      { t: 47.0, v: [-0.4, 0.25] },
      { t: 50, v: [-0.4, 0.25] },
    ],
    yaw: [
      { t: 0, v: KAEL_YAW },
      { t: 34.95, v: KAEL_YAW },
      { t: 35.3, v: KAEL_YAW + 0.35 },
      { t: 35.9, v: KAEL_YAW },
      { t: 50, v: KAEL_YAW },
    ],
    anims: [
      { clip: ANIM.idle, start: 0, end: 20.5, loop: true },
      { clip: ANIM.raise, start: 20.5, end: 22.5 },
      { clip: ANIM.blocking, start: 22.5, end: 29.4, loop: true },
      { clip: ANIM.run, start: 29.4, end: 30.75, loop: true, blend: 0.08 },
      { clip: ANIM.attackSliceDiag, start: 30.75, blend: 0.08 },
      { clip: ANIM.attackSliceHoriz, start: 31.95, blend: 0.08 },
      { clip: ANIM.attackChop, start: 33.05, blend: 0.08 },
      { clip: ANIM.block, start: 34.1, end: 34.95 },
      { clip: ANIM.hitB, start: 34.95, blend: 0.05 },
      { clip: ANIM.land, start: 37.55, blend: 0.1 },
      { clip: ANIM.blocking, start: 38.35, end: 43.3, loop: true },
      { clip: ANIM.powerUp, start: 43.3, end: 45.9, blend: 0.15 },
      { clip: ANIM.blocking, start: 45.9, end: 46.05 },
      { clip: ANIM.run, start: 46.05, end: 47.0, loop: true, blend: 0.06 },
      { clip: ANIM.attackStab, start: 47.0, blend: 0.06 },
    ],
  },

  varen: {
    pos: [
      { t: 0, v: VAREN_HOME },
      { t: 24.5, v: VAREN_HOME },
      { t: 27.3, v: [1.7, -0.5] },
      { t: 30.5, v: [0.9, 0.2] },
      { t: 33.55, v: [0.9, 0.2] },
      { t: 33.95, v: [1.45, -0.05] },
      { t: 34.6, v: [1.0, 0.15] },
      { t: 41.5, v: [1.0, 0.15] },
      { t: 43.2, v: [-0.5, 0.3] },
      { t: 50, v: [-0.5, 0.3] },
    ],
    yaw: [
      { t: 0, v: VAREN_YAW },
      { t: 24.5, v: VAREN_YAW },
      { t: 33.55, v: VAREN_YAW },
      { t: 33.95, v: VAREN_YAW + 0.5 },
      { t: 34.6, v: VAREN_YAW },
      { t: 50, v: VAREN_YAW },
    ],
    anims: [
      { clip: ANIM.idle2h, start: 0, end: 22.5, loop: true },
      { clip: ANIM.block, start: 22.5, end: 23.6 },
      { clip: ANIM.idle2h, start: 23.6, end: 24.5 },
      { clip: ANIM.walk, start: 24.5, end: 27.3, loop: true, timeScale: 0.85 },
      { clip: ANIM.idle2h, start: 27.3, end: 28.5 },
      { clip: ANIM.walk, start: 28.5, end: 30.5, loop: true, timeScale: 0.7 },
      { clip: ANIM.block, start: 30.5, end: 31.2 },
      { clip: ANIM.blockHit, start: 31.2, blend: 0.05 },
      { clip: ANIM.blockHit, start: 32.45, blend: 0.05 },
      { clip: ANIM.dodgeBack, start: 33.55, blend: 0.05 },
      { clip: ANIM.attack2hSlice, start: 34.05, blend: 0.08 },
      { clip: ANIM.idle2h, start: 35.3, end: 41.5, loop: true },
      { clip: ANIM.walk, start: 41.5, end: 43.2, loop: true, timeScale: 0.9 },
      { clip: ANIM.idle2h, start: 43.2, end: 46.9, loop: true },
      { clip: ANIM.block, start: 46.9, end: 50 },
    ],
  },

  impacts: [
    { t: 31.25, pos: [0.1, 1.25, 0.2], power: 1.0 },
    { t: 32.5, pos: [0.15, 1.2, 0.15], power: 0.8 },
    { t: 34.95, pos: [-0.35, 1.15, 0.25], power: 1.1 },
  ],
  debrisBursts: [{ t: 35.0, pos: [-0.7, 0.15, 0.35] }],
  dustBursts: [{ t: 37.65, pos: [-2.65, 0.1, 0.45] }],
  warningPulses: [{ t: 10.8, dur: 0.9 }],
  powerUp: { start: 43.3, end: 45.9 },
  combatHudFrom: 24.5,
  hp: {
    kael: [
      { t: 0, v: 1 },
      { t: 34.9, v: 1 },
      { t: 35.3, v: 0.55 },
    ],
    varen: [
      { t: 0, v: 1 },
      { t: 32.5, v: 1 },
      { t: 33.0, v: 0.92 },
    ],
  },
};

export const TEST_TIMELINE: Timeline = {
  fps: 30,
  durationSec: 10,
  shots: [
    {
      id: "T1",
      start: 0,
      end: 4,
      camera: {
        pos: [
          { t: 0, v: [0.4, 1.7, 5.2] },
          { t: 4, v: [0.2, 1.6, 4.9] },
        ],
        look: [{ t: 0, v: [0, 1.0, 0.1] }],
      },
    },
    {
      id: "T2",
      start: 4,
      end: 7,
      camera: {
        pos: [{ t: 4, v: [0.9, 1.5, 4.0] }],
        look: [{ t: 4, v: [0.1, 1.1, 0] }],
      },
    },
    {
      id: "T3",
      start: 7,
      end: 10,
      camera: {
        pos: [{ t: 7, v: [-0.7, 1.5, 4.4] }],
        look: [{ t: 7, v: [-0.4, 1.0, 0.1] }],
      },
    },
  ],
  kael: {
    pos: [
      { t: 0, v: [-1.5, 0.3] },
      { t: 2, v: [-1.5, 0.3] },
      { t: 4, v: [-0.6, 0.15] },
      { t: 7.55, v: [-0.55, 0.15] },
      { t: 8.6, v: [-1.2, 0.2] },
      { t: 10, v: [-1.2, 0.2] },
    ],
    yaw: [
      { t: 0, v: KAEL_YAW },
      { t: 10, v: KAEL_YAW },
    ],
    anims: [
      { clip: ANIM.idle, start: 0, end: 2, loop: true },
      { clip: ANIM.walk, start: 2, end: 4, loop: true },
      { clip: ANIM.idle, start: 4, end: 5.0 },
      { clip: ANIM.attackSliceDiag, start: 5.0, blend: 0.08 },
      { clip: ANIM.block, start: 6.2, end: 7.5 },
      { clip: ANIM.blockHit, start: 7.55, blend: 0.05 },
      { clip: ANIM.hitA, start: 8.5, blend: 0.08 },
      { clip: ANIM.idle, start: 9.3, end: 10 },
    ],
  },
  varen: {
    pos: [
      { t: 0, v: [1.1, 0.1] },
      { t: 10, v: [1.1, 0.1] },
    ],
    yaw: [
      { t: 0, v: VAREN_YAW },
      { t: 10, v: VAREN_YAW },
    ],
    anims: [
      { clip: ANIM.idle2h, start: 0, end: 5.3, loop: true },
      { clip: ANIM.block, start: 5.3, end: 5.6 },
      { clip: ANIM.blockHit, start: 5.6, blend: 0.05 },
      { clip: ANIM.attack2hSlice, start: 6.9, blend: 0.1 },
      { clip: ANIM.idle2h, start: 8.2, end: 10, loop: true },
    ],
  },
  impacts: [
    { t: 5.6, pos: [0.05, 1.2, 0.15], power: 1.0 },
    { t: 7.6, pos: [-0.35, 1.15, 0.15], power: 0.8 },
  ],
  debrisBursts: [],
  dustBursts: [],
  warningPulses: [],
  powerUp: null,
  combatHudFrom: 0,
  hp: {
    kael: [
      { t: 0, v: 1 },
      { t: 8.5, v: 1 },
      { t: 8.9, v: 0.7 },
    ],
    varen: [{ t: 0, v: 1 }],
  },
};

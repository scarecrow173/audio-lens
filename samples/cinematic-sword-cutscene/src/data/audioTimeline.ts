// Audio event placement on the master/test timelines.
// Single source of truth shared by <AudioTrack/> (Remotion) and
// scripts/audio/gen-audio.ts (out/metadata/audio.json).
//
// All times are SECONDS of source timeline. Bug variants keep the same
// audio track — bug profiles remap only the rendered video frame, so
// audio stays identical across variants by design.

export type AudioEvent = {
  id: string;
  type: 'voice' | 'sfx' | 'ambience' | 'bgm';
  // Path relative to the publicDir (assets/).
  file: string;
  startSec: number;
  durationSec: number;
  volume: number;
  speaker?: 'KAEL' | 'VAREN';
  loop?: boolean;
};

const voice = (
  id: string,
  speaker: 'KAEL' | 'VAREN',
  startSec: number,
  durationSec: number
): AudioEvent => ({
  id,
  type: 'voice',
  file: `audio/generated/voice/${id}.wav`,
  startSec,
  durationSec,
  volume: 1,
  speaker,
});

const sfx = (
  id: string,
  file: string,
  startSec: number,
  durationSec: number,
  volume: number
): AudioEvent => ({
  id,
  type: 'sfx',
  file: `audio/generated/sfx/${file}.wav`,
  startSec,
  durationSec,
  volume,
});

// Footstep helper: alternate the two variants across a time range.
const steps = (
  prefix: string,
  from: number,
  to: number,
  every: number,
  vol: number
): AudioEvent[] => {
  const out: AudioEvent[] = [];
  let i = 0;
  for (let t = from; t < to; t += every) {
    out.push(
      sfx(`${prefix}_${i}`, i % 2 ? 'footstep_b' : 'footstep_a', +t.toFixed(3), 0.2, vol)
    );
    i++;
  }
  return out;
};

export const AUDIO_MASTER: AudioEvent[] = [
  // --- voice (durations measured from generated files) ---
  voice('kael_01', 'KAEL', 5.3, 2.8),
  voice('varen_01', 'VAREN', 9.15, 5.3),
  voice('kael_02', 'KAEL', 14.1, 2.8),
  voice('varen_02', 'VAREN', 17.4, 3.4),
  voice('varen_03', 'VAREN', 41.6, 2.4),

  // --- dialogue blips (subtitle box appearing) ---
  sfx('ui_dlg_0', 'ui_dialogue', 4.55, 0.12, 0.25),
  sfx('ui_dlg_1', 'ui_dialogue', 9.05, 0.12, 0.25),
  sfx('ui_dlg_2', 'ui_dialogue', 13.55, 0.12, 0.25),
  sfx('ui_dlg_3', 'ui_dialogue', 17.05, 0.12, 0.25),
  sfx('ui_dlg_4', 'ui_dialogue', 41.55, 0.12, 0.25),

  // --- HUD appears at combat start ---
  sfx('ui_hud', 'ui_select', 24.5, 0.09, 0.4),

  // --- VAREN advance (walk cycles) ---
  ...steps('stepV_a', 24.6, 27.3, 0.55, 0.5),
  ...steps('stepV_b', 28.6, 30.5, 0.5, 0.55),
  ...steps('stepV_c', 41.6, 43.2, 0.55, 0.5),

  // --- KAEL rush-in ---
  ...steps('stepK', 29.5, 30.7, 0.27, 0.45),

  // --- combat ---
  sfx('swing_0', 'swing_light', 30.95, 0.32, 0.8),
  sfx('clash_0', 'clash', 31.25, 0.75, 0.95),
  sfx('swing_1', 'swing_light', 32.15, 0.32, 0.8),
  sfx('clash_1', 'clash', 32.5, 0.75, 0.9),
  sfx('swing_2', 'swing_light', 33.3, 0.32, 0.75),
  sfx('swing_3', 'swing_heavy', 34.4, 0.48, 0.9),
  sfx('impact_0', 'impact_heavy', 34.95, 0.9, 1),
  sfx('debris_0', 'debris', 35.05, 0.8, 0.7),
  sfx('land_0', 'landing', 37.65, 0.55, 0.85),

  // --- power-up ---
  sfx('powerup_0', 'powerup', 43.35, 2.6, 0.85),

  // --- final rush + VAREN guard ---
  sfx('swing_4', 'swing_heavy', 46.72, 0.48, 0.8),
  sfx('block_0', 'block', 46.95, 0.4, 0.7),

  // --- title ---
  sfx('title_0', 'title_impact', 47.2, 2.2, 0.95),

  // --- beds ---
  {
    id: 'ambience',
    type: 'ambience',
    file: 'audio/generated/ambience/ambience_loop.wav',
    startSec: 0,
    durationSec: 24,
    volume: 0.45,
    loop: true,
  },
  {
    id: 'bgm',
    type: 'bgm',
    file: 'audio/generated/bgm/bgm.wav',
    startSec: 0,
    durationSec: 50,
    volume: 0.6,
  },
];

// 10s technical test: same style, mapped onto its own timeline.
export const AUDIO_TEST: AudioEvent[] = [
  ...steps('tstepK', 2.05, 3.95, 0.4, 0.45),
  sfx('tswing_0', 'swing_light', 5.15, 0.32, 0.8),
  sfx('tclash_0', 'clash', 5.6, 0.75, 0.95),
  sfx('tswing_1', 'swing_heavy', 7.15, 0.48, 0.9),
  sfx('tblock_0', 'block', 7.55, 0.4, 0.8),
  sfx('timpact_0', 'impact_heavy', 8.55, 0.9, 0.9),
  {
    id: 'ambience',
    type: 'ambience',
    file: 'audio/generated/ambience/ambience_loop.wav',
    startSec: 0,
    durationSec: 24,
    volume: 0.45,
    loop: true,
  },
];

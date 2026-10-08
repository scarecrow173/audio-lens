// Audio-side bug variants.
//
// Unlike BugProfile (src/bugs/bugs.ts), which remaps the rendered video
// frame while audio stays identical, an AudioBugProfile leaves the video
// untouched and replaces the soundtrack: gen-bug-audio.ts premixes the
// master audio timeline into one WAV, then applies each bug as a
// deterministic DSP transform. The composition then plays that single
// pre-mixed file instead of the per-event AudioTrack.

export type AudioBugProfile =
  // Whole soundtrack drops to digital silence for a window.
  | {type: 'dropout'; startSec: number; endSec: number}
  // Entire soundtrack shifted: >0 audio is late, <0 audio is early.
  | {type: 'desync'; offsetSec: number}
  // Digital clipping: boost by gainDb, then hard-clip at full scale.
  | {type: 'clipping'; gainDb: number}
  // Unintended reverb send on the master bus (hall-like tail on everything).
  | {type: 'reverb'; wet: number; decaySec: number}
  // Wrong EQ/filter on the master bus for the whole duration.
  | {type: 'filter'; kind: 'lowpass' | 'highpass' | 'bandpass'; cutoffHz: number; q?: number}
  // Buffer-underrun style clicks/pops scattered in a window.
  | {type: 'clicks'; seed: number; count: number; startSec: number; endSec: number};

export type AudioBugVariant = {
  // Variant id; the pre-mixed file is assets/audio/generated/master/mix-<id>.wav
  id: string;
  composition: string;
  // Scenario dir under samples/ where the rendered MP4 is written.
  dir: string;
  bug: AudioBugProfile;
  description: string;
};

export const audioMixFile = (id: string): string =>
  `audio/generated/master/mix-${id}.wav`;

export const AUDIO_BUG_VARIANTS: readonly AudioBugVariant[] = [
  {
    id: 'audio-dropout',
    composition: 'GameCaptureAudioDropout',
    dir: 'dropout',
    bug: {type: 'dropout', startSec: 32.0, endSec: 33.0},
    description: '32.0-33.0s soundtrack drops to silence mid sword-combo',
  },
  {
    id: 'audio-desync',
    composition: 'GameCaptureAudioDesync',
    dir: 'desync',
    bug: {type: 'desync', offsetSec: 0.25},
    description: 'soundtrack runs +0.25s late against the picture',
  },
  {
    id: 'audio-clip',
    composition: 'GameCaptureAudioClip',
    dir: 'clipping',
    bug: {type: 'clipping', gainDb: 14},
    description: '+14dB digital clipping on the whole soundtrack',
  },
  {
    id: 'audio-reverb',
    composition: 'GameCaptureAudioReverb',
    dir: 'reverb',
    bug: {type: 'reverb', wet: 0.6, decaySec: 0.9},
    description: 'unintended hall reverb on the master bus',
  },
  {
    id: 'audio-filter',
    composition: 'GameCaptureAudioFilter',
    dir: 'filter',
    bug: {type: 'filter', kind: 'lowpass', cutoffHz: 700},
    description: '700Hz low-pass muffle on the whole soundtrack',
  },
  {
    id: 'audio-clicks',
    composition: 'GameCaptureAudioClicks',
    dir: 'clicks',
    bug: {type: 'clicks', seed: 4242, count: 16, startSec: 8, endSec: 50},
    description: 'buffer-underrun clicks scattered over 8-50s',
  },
];

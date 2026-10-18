import type React from "react";
import { Audio, Sequence, staticFile, useVideoConfig } from "remotion";
import type { AudioEvent } from "../data/audioTimeline";

// Places every audio event at its absolute source-time position.
// Deliberately not driven by the bug-remapped frame: video-only diffs
// stay attributable. Audio bug variants do not use this component — they
// play a pre-mixed WAV instead (see GameCapture's audioMix prop).
export const AudioTrack: React.FC<{ events: AudioEvent[] }> = ({ events }) => {
  const { fps, durationInFrames } = useVideoConfig();
  return (
    <>
      {events.map((e) => {
        const from = Math.round(e.startSec * fps);
        const dur = e.loop ? durationInFrames - from : Math.ceil(e.durationSec * fps);
        return (
          <Sequence key={e.id} from={from} durationInFrames={Math.max(1, dur)} layout="none">
            <Audio src={staticFile(e.file)} volume={e.volume} loop={e.loop} />
          </Sequence>
        );
      })}
    </>
  );
};

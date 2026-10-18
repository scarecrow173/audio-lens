import type React from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import type { BugProfile } from "../bugs/bugs";
import { shotAt, type Timeline } from "../data/shots";
import { activeClipNames } from "../three/characters/CharacterAnimationController";
import { impactFlashAt } from "../three/combat/SwordCollision";
import { useGameTime, useRemappedFrame } from "../utils/frameTime";

// Development-only diagnostics. Rendered via the `debug` prop — off in
// production outputs.
export const DebugOverlay: React.FC<{ timeline: Timeline; bug: BugProfile }> = ({
  timeline,
  bug,
}) => {
  const outFrame = useCurrentFrame();
  const srcFrame = useRemappedFrame();
  const { fps } = useVideoConfig();
  const t = useGameTime();
  const shot = shotAt(timeline, t);
  const isCut = timeline.shots.some((s) => Math.abs(t - s.start) < 1 / fps);
  const flash = impactFlashAt(timeline.impacts, t);
  const warn = timeline.warningPulses.some((w) => t >= w.t && t < w.t + w.dur);
  const power = timeline.powerUp && t >= timeline.powerUp.start && t <= timeline.powerUp.end;

  const lum: string[] = [];
  if (flash.intensity > 0.05) lum.push(`IMPACT ${flash.intensity.toFixed(2)}`);
  if (warn) lum.push("WARNING");
  if (power) lum.push("POWERUP");
  if (shot?.blackout) lum.push("BLACKOUT");

  const line: React.CSSProperties = {
    fontFamily: "Consolas, monospace",
    fontSize: 9,
    color: "#9fe8ff",
    textShadow: "0 0 3px #000",
    lineHeight: 1.5,
  };

  return (
    <div style={{ position: "absolute", left: 8, bottom: 8, pointerEvents: "none" }}>
      <div style={line}>
        out_f={outFrame} src_f={srcFrame} t={t.toFixed(2)}s shot={shot?.id ?? "-"}
        {isCut ? " [CUT]" : ""}
      </div>
      <div style={line}>KAEL: {activeClipNames(timeline.kael.anims, t)}</div>
      <div style={line}>VAREN: {activeClipNames(timeline.varen.anims, t)}</div>
      <div style={line}>
        bug={bug.type}
        {lum.length ? ` | ${lum.join(" ")}` : ""}
      </div>
    </div>
  );
};

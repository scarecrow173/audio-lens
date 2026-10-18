import type React from "react";
import { AbsoluteFill } from "remotion";
import { dialogueAt } from "../data/dialogue";
import { shotAt, type Timeline } from "../data/shots";
import { evalKeys, useGameTime } from "../utils/frameTime";

const BAR_BLOCKS = 10;

const HpBar: React.FC<{ value: number; color: string }> = ({ value, color }) => {
  const filled = Math.round(value * BAR_BLOCKS);
  return (
    <span style={{ letterSpacing: 1, fontSize: 11 }}>
      {new Array(BAR_BLOCKS).fill(0).map((_, i) => (
        <span key={i} style={{ color: i < filled ? color : "#2a3542" }}>
          █
        </span>
      ))}
    </span>
  );
};

const nameStyle: React.CSSProperties = {
  fontFamily: '"Segoe UI", "Yu Gothic", monospace',
  fontSize: 12,
  fontWeight: 700,
  letterSpacing: 2,
  textShadow: "0 0 4px #000",
};

// Minimal game HUD: HP bars + lock-on marker during combat, dialogue lines.
export const GameHUD: React.FC<{ timeline: Timeline }> = ({ timeline }) => {
  const t = useGameTime();
  const combat = t >= timeline.combatHudFrom;
  const dlg = dialogueAt(timeline, t);
  const shot = shotAt(timeline, t);
  const kaelHp = evalKeys(timeline.hp.kael, t, (x) => x);
  const varenHp = evalKeys(timeline.hp.varen, t, (x) => x);

  return (
    <AbsoluteFill style={{ pointerEvents: "none" }}>
      {combat && (
        <>
          <div style={{ position: "absolute", top: 10, left: 12 }}>
            <div style={{ ...nameStyle, color: "#6ff2ff" }}>KAEL</div>
            <div style={{ marginTop: 2 }}>
              <HpBar value={kaelHp} color="#46d8ec" />
            </div>
          </div>
          <div style={{ position: "absolute", top: 10, right: 12, textAlign: "right" }}>
            <div style={{ ...nameStyle, color: "#ff5560" }}>VAREN</div>
            <div style={{ marginTop: 2 }}>
              <HpBar value={varenHp} color="#e0424e" />
            </div>
          </div>
          <div
            style={{
              position: "absolute",
              bottom: 10,
              right: 14,
              ...nameStyle,
              color: "#9fb6c6",
              fontSize: 10,
              opacity: 0.8,
            }}
          >
            LOCK ◈
          </div>
        </>
      )}
      {dlg && (
        <div
          style={{
            position: "absolute",
            bottom: 16,
            left: 0,
            right: 0,
            textAlign: "center",
          }}
        >
          <div
            style={{
              display: "inline-block",
              padding: "4px 14px",
              background: "rgba(4,8,14,0.72)",
              borderRadius: 3,
              fontFamily: '"Yu Gothic", "Meiryo", "Segoe UI", sans-serif',
              fontSize: 14,
              color: "#e8f1f5",
              textShadow: "0 1px 2px #000",
            }}
          >
            <span
              style={{
                color: dlg.speaker === "KAEL" ? "#6ff2ff" : "#ff5560",
                fontWeight: 700,
                marginRight: 8,
              }}
            >
              {dlg.speaker}
            </span>
            {dlg.text}
          </div>
        </div>
      )}
      {shot?.blackout && (
        <AbsoluteFill style={{ background: "#000" }}>
          {shot.title && t >= shot.title.from && t <= shot.title.to && (
            <div
              style={{
                height: "100%",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                fontFamily: '"Segoe UI", "Yu Gothic", sans-serif',
              }}
            >
              <div
                style={{
                  color: "#cfe3ec",
                  fontSize: 26,
                  letterSpacing: 8,
                  fontWeight: 300,
                }}
              >
                {shot.title.line1}
              </div>
              <div
                style={{
                  color: "#5d7688",
                  fontSize: 12,
                  letterSpacing: 5,
                  marginTop: 10,
                }}
              >
                {shot.title.line2}
              </div>
            </div>
          )}
        </AbsoluteFill>
      )}
    </AbsoluteFill>
  );
};

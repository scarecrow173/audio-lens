import { useFrame } from "@react-three/fiber";
import type React from "react";
import { useMemo, useRef } from "react";
import * as THREE from "three";
import type { CharacterTrack } from "../../data/shots";
import { useGameTime } from "../../utils/frameTime";
import type { CharacterHandle } from "../characters/KayKitCharacter";
import { trailActive } from "./CombatChoreography";

const SAMPLES = 9;
const WINDOW = 0.28; // seconds of blade history

// Ribbon trail rebuilt each frame from real blade base/tip positions
// sampled over the recent past (pure function of time -> deterministic).
export const SwordTrail: React.FC<{
  character: React.RefObject<CharacterHandle | null>;
  track: CharacterTrack;
  color: string;
}> = ({ character, track, color }) => {
  const t = useGameTime();
  const meshRef = useRef<THREE.Mesh>(null);
  const matRef = useRef<THREE.MeshBasicMaterial>(null);

  const geo = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.BufferAttribute(new Float32Array(SAMPLES * 2 * 3), 3));
    const idx: number[] = [];
    for (let i = 0; i < SAMPLES - 1; i++) {
      const a = i * 2;
      idx.push(a, a + 1, a + 2, a + 1, a + 3, a + 2);
    }
    g.setIndex(idx);
    return g;
  }, []);

  useFrame(() => {
    const mesh = meshRef.current;
    const h = character.current;
    if (!mesh || !h) return;
    const active = trailActive(track, t);
    mesh.visible = active;
    if (matRef.current) matRef.current.opacity = active ? 0.55 : 0;
    if (!active) return;

    const attr = geo.getAttribute("position") as THREE.BufferAttribute;
    const arr = attr.array as Float32Array;
    for (let i = 0; i < SAMPLES; i++) {
      const st = t - (i / (SAMPLES - 1)) * WINDOW;
      const pts = st >= 0 ? h.weaponPointsAt(Math.max(0, st)) : null;
      if (pts) {
        arr[i * 6] = pts.base.x;
        arr[i * 6 + 1] = pts.base.y;
        arr[i * 6 + 2] = pts.base.z;
        arr[i * 6 + 3] = pts.tip.x;
        arr[i * 6 + 4] = pts.tip.y;
        arr[i * 6 + 5] = pts.tip.z;
      }
    }
    attr.needsUpdate = true;
    // restore current pose after sampling the past
    h.poseAt(t);
  });

  return (
    <mesh ref={meshRef} geometry={geo} frustumCulled={false}>
      <meshBasicMaterial
        ref={matRef}
        color={color}
        transparent
        opacity={0}
        side={THREE.DoubleSide}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </mesh>
  );
};

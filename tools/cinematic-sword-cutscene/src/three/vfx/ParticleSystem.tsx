import { useFrame } from "@react-three/fiber";
import type React from "react";
import { useMemo, useRef } from "react";
import * as THREE from "three";
import { useGameTime } from "../../utils/frameTime";
import { hashRandom } from "../../utils/seededRandom";

export type BurstSpec = {
  t: number;
  pos: readonly [number, number, number];
};

export type BurstParams = {
  count: number;
  seed: number;
  color: string;
  speedMin: number;
  speedMax: number;
  // horizontal bias: 0 = sphere, 1 = mostly horizontal disk
  flatness: number;
  gravity: number;
  life: number;
  size: number;
  fadePower?: number;
  // upward bias 0..1
  upBias?: number;
};

let spriteTexture: THREE.Texture | null = null;
export const getSpriteTexture = (): THREE.Texture => {
  if (spriteTexture) return spriteTexture;
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const ctx = c.getContext("2d");
  if (!ctx) throw new Error("2d canvas context is unavailable");
  const g = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
  g.addColorStop(0, "rgba(255,255,255,1)");
  g.addColorStop(0.4, "rgba(255,255,255,0.6)");
  g.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, 64, 64);
  spriteTexture = new THREE.CanvasTexture(c);
  return spriteTexture;
};

// Deterministic particle burst: particle i of event e has a hashed
// direction/speed; world position is evaluated purely from (t - event.t).
export const ParticleBurst: React.FC<{
  events: readonly BurstSpec[];
  params: BurstParams;
}> = ({ events, params }) => {
  const t = useGameTime();
  const pointsRef = useRef<THREE.Points>(null);
  const matRef = useRef<THREE.PointsMaterial>(null);

  const { count, seed } = params;
  const geo = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.BufferAttribute(new Float32Array(count * 3), 3));
    return g;
  }, [count]);

  // Precompute per-particle velocity deterministically.
  const vels = useMemo(() => {
    const arr = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      const a = hashRandom(seed, i * 3) * Math.PI * 2;
      const b = (hashRandom(seed, i * 3 + 1) - 0.5) * Math.PI * (1 - params.flatness);
      const sp =
        params.speedMin + hashRandom(seed, i * 3 + 2) * (params.speedMax - params.speedMin);
      const up = params.upBias ?? 0;
      arr[i * 3] = Math.cos(a) * Math.cos(b) * sp;
      arr[i * 3 + 1] = (Math.sin(b) + up) * sp;
      arr[i * 3 + 2] = Math.sin(a) * Math.cos(b) * sp;
    }
    return arr;
  }, [count, seed, params]);

  useFrame(() => {
    const pts = pointsRef.current;
    if (!pts) return;
    const attr = geo.getAttribute("position") as THREE.BufferAttribute;
    const arr = attr.array as Float32Array;
    arr.fill(0);
    let opacity = 0;
    let visible = false;

    for (const ev of events) {
      const dt = t - ev.t;
      if (dt < 0 || dt > params.life) continue;
      visible = true;
      const fade = (1 - dt / params.life) ** (params.fadePower ?? 1.6);
      opacity = Math.max(opacity, fade);
      for (let i = 0; i < count; i++) {
        arr[i * 3] = ev.pos[0] + vels[i * 3] * dt;
        arr[i * 3 + 1] = ev.pos[1] + vels[i * 3 + 1] * dt - 0.5 * params.gravity * dt * dt;
        arr[i * 3 + 2] = ev.pos[2] + vels[i * 3 + 2] * dt;
      }
    }
    attr.needsUpdate = true;
    pts.visible = visible;
    if (matRef.current) matRef.current.opacity = opacity;
  });

  return (
    <points ref={pointsRef} geometry={geo} frustumCulled={false}>
      <pointsMaterial
        ref={matRef}
        color={params.color}
        size={params.size}
        map={getSpriteTexture()}
        transparent
        opacity={0}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        sizeAttenuation
      />
    </points>
  );
};

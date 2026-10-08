import React, {useMemo, useRef} from 'react';
import * as THREE from 'three';
import {useFrame} from '@react-three/fiber';
import {clamp, useGameTime} from '../../utils/frameTime';
import {hashRandom} from '../../utils/seededRandom';
import {getSpriteTexture} from './ParticleSystem';
import {CharacterTrack} from '../../data/shots';
import {positionAt} from '../characters/CharacterAnimationController';

const COUNT = 130;
const SEED = 4451;
const RATE = 65; // particles per second

// Cyan motes spiralling up around KAEL during the power-up window.
export const PowerParticles: React.FC<{
  window: {start: number; end: number} | null;
  track: CharacterTrack;
}> = ({window: win, track}) => {
  const t = useGameTime();
  const ref = useRef<THREE.Points>(null);
  const matRef = useRef<THREE.PointsMaterial>(null);

  const geo = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(COUNT * 3), 3));
    return g;
  }, []);

  useFrame(() => {
    const pts = ref.current;
    if (!pts) return;
    const attr = geo.getAttribute('position') as THREE.BufferAttribute;
    const arr = attr.array as Float32Array;
    arr.fill(-999);
    let opacity = 0;

    if (win && t >= win.start - 0.1 && t <= win.end + 0.5) {
      opacity = t <= win.end ? clamp((t - win.start) / 0.6) : Math.max(0, 1 - (t - win.end) / 0.5);
      const p = positionAt(track, Math.min(t, win.end));
      for (let i = 0; i < COUNT; i++) {
        const spawn = win.start + (i / RATE);
        const age = t - spawn;
        if (age < 0 || age > 1.1) continue;
        const a = hashRandom(SEED, i) * Math.PI * 2 + age * 3.2;
        const r = 0.35 + hashRandom(SEED, i + 77) * 0.5 - age * 0.15;
        arr[i * 3] = p.x + Math.cos(a) * r;
        arr[i * 3 + 1] = 0.05 + age * (1.1 + hashRandom(SEED, i + 31) * 0.9);
        arr[i * 3 + 2] = p.z + Math.sin(a) * r;
      }
    }
    attr.needsUpdate = true;
    pts.visible = opacity > 0;
    if (matRef.current) matRef.current.opacity = opacity * 0.95;
  });

  return (
    <points ref={ref} geometry={geo} frustumCulled={false}>
      <pointsMaterial
        ref={matRef}
        color="#4df0ff"
        size={0.07}
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

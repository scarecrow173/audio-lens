import React, {useMemo, useRef} from 'react';
import * as THREE from 'three';
import {useFrame, useThree} from '@react-three/fiber';
import {useGameTime} from '../../utils/frameTime';
import {hashRandom} from '../../utils/seededRandom';
import {getSpriteTexture} from './ParticleSystem';

const COUNT = 7;
const SEED = 3313;

// Large, faint billboard fog blobs drifting through the arena.
export const FogParticles: React.FC = () => {
  const t = useGameTime();
  const camera = useThree((s) => s.camera);
  const groupRef = useRef<THREE.Group>(null);

  const blobs = useMemo(
    () =>
      new Array(COUNT).fill(0).map((_, i) => ({
        x: (hashRandom(SEED, i) - 0.5) * 16,
        y: 0.8 + hashRandom(SEED, i + 50) * 2.2,
        z: -4 + hashRandom(SEED, i + 90) * 8,
        scale: 4 + hashRandom(SEED, i + 130) * 5,
        drift: 0.08 + hashRandom(SEED, i + 170) * 0.12,
        phase: hashRandom(SEED, i + 210) * Math.PI * 2,
      })),
    []
  );

  useFrame(() => {
    const g = groupRef.current;
    if (!g) return;
    g.children.forEach((child, i) => {
      const b = blobs[i];
      child.position.set(
        b.x + Math.sin(t * 0.1 + b.phase) * 1.5 + t * b.drift * 0.3,
        b.y + Math.sin(t * 0.07 + b.phase * 2) * 0.25,
        b.z
      );
      child.quaternion.copy(camera.quaternion);
    });
  });

  return (
    <group ref={groupRef}>
      {blobs.map((b, i) => (
        <mesh key={i}>
          <planeGeometry args={[b.scale, b.scale * 0.55]} />
          <meshBasicMaterial
            map={getSpriteTexture()}
            color="#12232e"
            transparent
            opacity={0.35}
            depthWrite={false}
          />
        </mesh>
      ))}
    </group>
  );
};

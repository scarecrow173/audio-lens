import React, {useMemo, useRef} from 'react';
import * as THREE from 'three';
import {useFrame} from '@react-three/fiber';
import {useGameTime} from '../../utils/frameTime';
import {hashRandom} from '../../utils/seededRandom';
import {getSpriteTexture} from './ParticleSystem';

const COUNT = 220;
const SEED = 1207;
const BOX = {x: 14, y: 5, z: 11};

// Continuously drifting dust motes — position is a pure function of time.
export const AmbientDust: React.FC = () => {
  const t = useGameTime();
  const ref = useRef<THREE.Points>(null);

  const geo = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(COUNT * 3), 3));
    return g;
  }, []);

  const base = useMemo(() => {
    const arr = new Float32Array(COUNT * 3);
    const drift = new Float32Array(COUNT * 3);
    for (let i = 0; i < COUNT; i++) {
      arr[i * 3] = (hashRandom(SEED, i) - 0.5) * BOX.x;
      arr[i * 3 + 1] = hashRandom(SEED, i + 1000) * BOX.y;
      arr[i * 3 + 2] = (hashRandom(SEED, i + 2000) - 0.5) * BOX.z - 2;
      drift[i * 3] = (hashRandom(SEED, i + 3000) - 0.5) * 0.14;
      drift[i * 3 + 1] = -0.02 - hashRandom(SEED, i + 4000) * 0.05;
      drift[i * 3 + 2] = (hashRandom(SEED, i + 5000) - 0.5) * 0.1;
    }
    return {arr, drift};
  }, []);

  useFrame(() => {
    const attr = geo.getAttribute('position') as THREE.BufferAttribute;
    const out = attr.array as Float32Array;
    for (let i = 0; i < COUNT; i++) {
      // wrap each axis into its box (z box is centered at -2)
      const y = base.arr[i * 3 + 1] + base.drift[i * 3 + 1] * t;
      out[i * 3 + 1] = ((y % BOX.y) + BOX.y) % BOX.y;
      const x = base.arr[i * 3] + base.drift[i * 3] * t;
      out[i * 3] = ((((x + BOX.x / 2) % BOX.x) + BOX.x) % BOX.x) - BOX.x / 2;
      const z = base.arr[i * 3 + 2] + base.drift[i * 3 + 2] * t + 2;
      out[i * 3 + 2] = ((((z + BOX.z / 2) % BOX.z) + BOX.z) % BOX.z) - BOX.z / 2 - 2;
    }
    attr.needsUpdate = true;
  });

  return (
    <points ref={ref} geometry={geo} frustumCulled={false}>
      <pointsMaterial
        color="#5d7b8c"
        size={0.05}
        map={getSpriteTexture()}
        transparent
        opacity={0.5}
        depthWrite={false}
        sizeAttenuation
      />
    </points>
  );
};

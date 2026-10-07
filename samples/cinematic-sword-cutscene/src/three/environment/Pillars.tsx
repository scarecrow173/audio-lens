import React, {useLayoutEffect, useMemo, useRef} from 'react';
import * as THREE from 'three';
import {createRng} from '../../utils/seededRandom';

// Instanced pillars (some intact, some broken/tilted) + ceiling beams.
export const Pillars: React.FC = () => {
  const {intact, broken, beams} = useMemo(() => {
    const rng = createRng(7703);
    const intactM: THREE.Matrix4[] = [];
    const brokenM: THREE.Matrix4[] = [];
    const beamM: THREE.Matrix4[] = [];
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion();
    const e = new THREE.Euler();
    const s = new THREE.Vector3(1, 1, 1);

    // two pillar rows flanking the arena
    for (const side of [-1, 1]) {
      for (let i = 0; i < 6; i++) {
        const x = side * (5.5 + rng() * 0.6);
        const z = -8 + i * 3.4 + (rng() - 0.5) * 0.8;
        if (rng() < 0.3) {
          const h = 0.8 + rng() * 1.6;
          e.set((rng() - 0.5) * 0.25, rng() * Math.PI, (rng() - 0.5) * 0.25);
          q.setFromEuler(e);
          m.compose(new THREE.Vector3(x, h / 2, z), q, s.set(1, h, 1));
          brokenM.push(m.clone());
        } else {
          const h = 7 + rng() * 1.5;
          e.set(0, rng() * Math.PI, 0);
          q.setFromEuler(e);
          m.compose(new THREE.Vector3(x, h / 2, z), q, s.set(1, h, 1));
          intactM.push(m.clone());
        }
      }
    }
    // background pillar cluster
    for (let i = 0; i < 8; i++) {
      const x = (rng() - 0.5) * 30;
      const z = -12 - rng() * 14;
      const h = 6 + rng() * 6;
      e.set(0, rng() * Math.PI, 0);
      q.setFromEuler(e);
      m.compose(new THREE.Vector3(x, h / 2, z), q, s.set(1.4, h, 1.4));
      intactM.push(m.clone());
    }
    // ceiling beams
    for (let i = 0; i < 5; i++) {
      const z = -9 + i * 4;
      e.set(0, (rng() - 0.5) * 0.15, 0);
      q.setFromEuler(e);
      m.compose(
        new THREE.Vector3((rng() - 0.5) * 3, 7.6 + rng(), z),
        q,
        s.set(1, 1, 1)
      );
      beamM.push(m.clone());
    }
    return {intact: intactM, broken: brokenM, beams: beamM};
  }, []);

  const pillarGeo = useMemo(() => new THREE.BoxGeometry(0.9, 1, 0.9), []);
  const beamGeo = useMemo(() => new THREE.BoxGeometry(26, 0.5, 0.7), []);

  const intactRef = useRef<THREE.InstancedMesh>(null);
  const brokenRef = useRef<THREE.InstancedMesh>(null);
  const beamRef = useRef<THREE.InstancedMesh>(null);

  useLayoutEffect(() => {
    const apply = (mesh: THREE.InstancedMesh | null, mats: THREE.Matrix4[]) => {
      if (!mesh) return;
      mats.forEach((mtx, i) => mesh.setMatrixAt(i, mtx));
      mesh.instanceMatrix.needsUpdate = true;
    };
    apply(intactRef.current, intact);
    apply(brokenRef.current, broken);
    apply(beamRef.current, beams);
  }, [intact, broken, beams]);

  return (
    <group>
      <instancedMesh ref={intactRef} args={[pillarGeo, undefined, intact.length]}>
        <meshStandardMaterial color="#1c2530" roughness={0.9} />
      </instancedMesh>
      <instancedMesh ref={brokenRef} args={[pillarGeo, undefined, broken.length]}>
        <meshStandardMaterial color="#232d3a" roughness={0.95} />
      </instancedMesh>
      <instancedMesh ref={beamRef} args={[beamGeo, undefined, beams.length]}>
        <meshStandardMaterial color="#151c26" roughness={0.95} />
      </instancedMesh>
    </group>
  );
};

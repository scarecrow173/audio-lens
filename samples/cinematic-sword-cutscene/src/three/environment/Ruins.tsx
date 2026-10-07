import React, {useLayoutEffect, useMemo, useRef} from 'react';
import * as THREE from 'three';
import {createRng} from '../../utils/seededRandom';

// Rubble, machinery blocks and distant structures filling out the
// collapsed-facility look.
export const Ruins: React.FC = () => {
  const {debrisM, machines, structures} = useMemo(() => {
    const rng = createRng(9021);
    const debrisM: THREE.Matrix4[] = [];
    const machines: {pos: [number, number, number]; size: [number, number, number]}[] = [];
    const structures: {pos: [number, number, number]; size: [number, number, number]}[] = [];
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion();
    const e = new THREE.Euler();
    const s = new THREE.Vector3();

    for (let i = 0; i < 90; i++) {
      const x = (rng() - 0.5) * 22;
      const z = (rng() - 0.5) * 16 - 2;
      const sc = 0.06 + rng() * 0.3;
      e.set(rng() * Math.PI, rng() * Math.PI, rng() * Math.PI);
      q.setFromEuler(e);
      m.compose(new THREE.Vector3(x, sc * 0.4, z), q, s.set(sc, sc, sc));
      debrisM.push(m.clone());
    }
    for (let i = 0; i < 6; i++) {
      const x = (rng() - 0.5) * 24;
      const z = -9 - rng() * 4;
      const h = 1 + rng() * 1.6;
      machines.push({pos: [x, h / 2, z], size: [1.2 + rng() * 1.6, h, 0.9 + rng()]});
    }
    for (let i = 0; i < 9; i++) {
      const x = (rng() - 0.5) * 44;
      const z = -18 - rng() * 14;
      const h = 3 + rng() * 8;
      structures.push({pos: [x, h / 2, z], size: [3 + rng() * 6, h, 2 + rng() * 4]});
    }
    return {debrisM, machines, structures};
  }, []);

  const debrisGeo = useMemo(() => new THREE.BoxGeometry(1, 1, 1), []);
  const debrisRef = useRef<THREE.InstancedMesh>(null);
  useLayoutEffect(() => {
    const mesh = debrisRef.current;
    if (!mesh) return;
    debrisM.forEach((mtx, i) => mesh.setMatrixAt(i, mtx));
    mesh.instanceMatrix.needsUpdate = true;
  }, [debrisM]);

  return (
    <group>
      <instancedMesh ref={debrisRef} args={[debrisGeo, undefined, debrisM.length]}>
        <meshStandardMaterial color="#1a222d" roughness={1} />
      </instancedMesh>
      {machines.map((mc, i) => (
        <group key={i} position={mc.pos}>
          <mesh>
            <boxGeometry args={mc.size} />
            <meshStandardMaterial color="#131a23" roughness={0.85} metalness={0.25} />
          </mesh>
          {/* status light strip */}
          <mesh position={[0, mc.size[1] / 2 - 0.12, mc.size[2] / 2 + 0.01]}>
            <boxGeometry args={[mc.size[0] * 0.7, 0.05, 0.02]} />
            <meshStandardMaterial
              color="#06222a"
              emissive={i % 3 === 0 ? '#a0222b' : '#0f7f8f'}
              emissiveIntensity={1.4}
            />
          </mesh>
        </group>
      ))}
      {structures.map((st, i) => (
        <mesh key={i} position={st.pos}>
          <boxGeometry args={st.size} />
          <meshStandardMaterial color="#0c1118" roughness={1} />
        </mesh>
      ))}
    </group>
  );
};

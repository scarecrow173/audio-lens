import React, {useMemo} from 'react';
import * as THREE from 'three';
import {createRng} from '../../utils/seededRandom';

// Dark facility floor + perimeter walls + raised platforms.
export const Floor: React.FC = () => {
  const platforms = useMemo(() => {
    const rng = createRng(4101);
    const list: {pos: [number, number, number]; size: [number, number, number]}[] = [];
    for (let i = 0; i < 7; i++) {
      const x = (rng() - 0.5) * 26;
      const z = -6 - rng() * 10;
      const h = 0.4 + rng() * 1.4;
      list.push({
        pos: [x, h / 2, z],
        size: [2 + rng() * 4, h, 2 + rng() * 3],
      });
    }
    return list;
  }, []);

  return (
    <group>
      {/* main floor */}
      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, 0, 0]}>
        <planeGeometry args={[60, 50]} />
        <meshStandardMaterial color="#141a24" roughness={0.95} metalness={0.05} />
      </mesh>
      {/* center arena plate */}
      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, 0.01, 0]}>
        <planeGeometry args={[14, 10]} />
        <meshStandardMaterial color="#1b2531" roughness={0.9} metalness={0.1} />
      </mesh>
      {/* glowing floor strips — luminance anchors + game readability */}
      {[-2.4, 0, 2.4].map((x) => (
        <mesh key={x} rotation={[-Math.PI / 2, 0, 0]} position={[x, 0.02, 0]}>
          <planeGeometry args={[0.07, 9]} />
          <meshStandardMaterial
            color="#0a2a33"
            emissive="#0f6d7d"
            emissiveIntensity={0.9}
            roughness={1}
          />
        </mesh>
      ))}
      {/* back wall */}
      <mesh position={[0, 4, -11]}>
        <boxGeometry args={[44, 9, 1]} />
        <meshStandardMaterial color="#10151d" roughness={1} />
      </mesh>
      {/* side walls */}
      <mesh position={[-15, 4, 0]}>
        <boxGeometry args={[1, 9, 30]} />
        <meshStandardMaterial color="#0e131a" roughness={1} />
      </mesh>
      <mesh position={[15, 4, 0]}>
        <boxGeometry args={[1, 9, 30]} />
        <meshStandardMaterial color="#0e131a" roughness={1} />
      </mesh>
      {/* collapsed platforms */}
      {platforms.map((p, i) => (
        <mesh key={i} position={p.pos}>
          <boxGeometry args={p.size} />
          <meshStandardMaterial color="#171f2a" roughness={0.95} />
        </mesh>
      ))}
    </group>
  );
};

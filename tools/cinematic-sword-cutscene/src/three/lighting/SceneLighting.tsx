import React, {useRef} from 'react';
import * as THREE from 'three';
import {useFrame} from '@react-three/fiber';
import {Timeline} from '../../data/shots';
import {clamp, useGameTime} from '../../utils/frameTime';

// One lighting rig for the whole video so shots feel like one game session.
export const SceneLighting: React.FC<{timeline: Timeline}> = ({timeline}) => {
  const t = useGameTime();
  const warningRef = useRef<THREE.PointLight>(null);
  const flashRef = useRef<THREE.PointLight>(null);
  const powerRef = useRef<THREE.PointLight>(null);
  const ambientRef = useRef<THREE.AmbientLight>(null);
  const keyRef = useRef<THREE.DirectionalLight>(null);

  useFrame(() => {
    // red warning pulse (SHOT 03)
    let warn = 0;
    for (const w of timeline.warningPulses) {
      const dt = t - w.t;
      if (dt >= 0 && dt < w.dur) {
        warn = Math.max(warn, Math.sin((dt / w.dur) * Math.PI));
      }
    }
    if (warningRef.current) warningRef.current.intensity = warn * 90;

    // impact flash lights
    let flash = 0;
    let flashPos: readonly [number, number, number] = [0, 1.2, 0];
    for (const imp of timeline.impacts) {
      const dt = t - imp.t;
      if (dt >= 0 && dt < 0.18) {
        const k = 1 - dt / 0.18;
        if (k * imp.power > flash) {
          flash = k * imp.power;
          flashPos = imp.pos;
        }
      }
    }
    if (flashRef.current) {
      flashRef.current.intensity = flash * 130;
      flashRef.current.position.set(...flashPos);
    }

    // power-up ramp (cyan, ~2s rise)
    let pw = 0;
    if (timeline.powerUp) {
      const {start, end} = timeline.powerUp;
      if (t >= start && t <= end + 0.6) {
        pw = t <= end ? clamp((t - start) / 1.8) : Math.max(0, 1 - (t - end) / 0.6);
      }
    }
    if (powerRef.current) powerRef.current.intensity = pw * 55;
    if (ambientRef.current) ambientRef.current.intensity = 1.35 + warn * 0.2 + pw * 0.3;
    if (keyRef.current) keyRef.current.intensity = 2.3 + pw * 0.45;
  });

  return (
    <group>
      <ambientLight ref={ambientRef} intensity={1.35} color="#40597a" />
      <hemisphereLight args={['#5a8fa3', '#1c2530', 0.85]} />
      <directionalLight
        ref={keyRef}
        position={[4, 9, 5]}
        intensity={2.3}
        color="#9adfe8"
      />
      <directionalLight position={[-6, 6, -4]} intensity={0.8} color="#4a6284" />
      {/* camera-side fill so faces never fall to black */}
      <directionalLight position={[0, 4, 12]} intensity={0.55} color="#3d5a70" />
      {/* warning light — back wall */}
      <pointLight
        ref={warningRef}
        position={[0, 6.5, -9.5]}
        color="#ff2233"
        intensity={0}
        distance={30}
        decay={1.6}
      />
      {/* impact flash light */}
      <pointLight
        ref={flashRef}
        position={[0, 1.2, 0]}
        color="#cfeaff"
        intensity={0}
        distance={9}
        decay={2}
      />
      {/* power-up glow at KAEL */}
      <pointLight
        ref={powerRef}
        position={[-2.7, 1.4, 0.45]}
        color="#39e6ff"
        intensity={0}
        distance={8}
        decay={2}
      />
      {/* low teal fill over the arena */}
      <pointLight position={[0, 3.2, 1.5]} color="#2a6a7d" intensity={16} distance={18} decay={2} />
      {/* overhead practicals along the arena so the room reads as lit */}
      <pointLight position={[-6, 6.8, -4]} color="#3d7f8f" intensity={10} distance={14} decay={2} />
      <pointLight position={[6, 6.8, -4]} color="#3d7f8f" intensity={10} distance={14} decay={2} />
    </group>
  );
};

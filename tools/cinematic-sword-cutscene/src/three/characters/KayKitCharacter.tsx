import React, {
  forwardRef,
  useImperativeHandle,
  useMemo,
  useRef,
} from 'react';
import * as THREE from 'three';
import {useFrame} from '@react-three/fiber';
import {clone as skeletonClone} from 'three/examples/jsm/utils/SkeletonUtils.js';
import {getGltf} from '../../assets';
import {CharacterManifest} from '../../data/kaykitManifest';
import {CharacterTrack} from '../../data/shots';
import {activeClipNames, activeClipsAt, positionAt} from './CharacterAnimationController';
import {useGameTime} from '../../utils/frameTime';
import {WeaponSpec, attachWeapon} from './WeaponAttachment';

export type CharacterHandle = {
  // Set the full pose (transform + animation) at source time t.
  poseAt: (t: number) => void;
  // World-space weapon base/tip after posing at t (used by sword trails).
  weaponPointsAt: (t: number) => {base: THREE.Vector3; tip: THREE.Vector3} | null;
  positionAt: (t: number) => THREE.Vector3;
  clipLabel: (t: number) => string;
};

type Props = {
  manifest: CharacterManifest;
  track: CharacterTrack;
  // Embedded attachment node names to keep visible (rest are hidden).
  showNodes: string[];
  externalWeapon?: WeaponSpec;
  // Local Y of the blade tip on embedded weapons.
  embeddedTipY?: number;
  rimColor: string;
};

const Inner: React.ForwardRefRenderFunction<CharacterHandle, Props> = (
  {manifest, track, showNodes, externalWeapon, embeddedTipY = 1.3, rimColor},
  ref
) => {
  const gltf = getGltf(manifest.file);
  const t = useGameTime();
  const groupRef = useRef<THREE.Group>(null);
  const markersRef = useRef<{base: THREE.Object3D; tip: THREE.Object3D} | null>(null);

  const model = useMemo(() => {
    const m = skeletonClone(gltf.scene);
    m.traverse((o) => {
      const mesh = o as THREE.Mesh;
      if (mesh.isMesh) {
        mesh.frustumCulled = false;
        mesh.castShadow = false;
        mesh.receiveShadow = false;
      }
    });
    // Loadout: hide every attachment node except the requested ones.
    for (const att of manifest.attachments) {
      for (const name of att.nodes) {
        const node = m.getObjectByName(name);
        if (node) node.visible = showNodes.includes(name);
      }
    }
    // faint emissive on shown weapons so blades read at 640x360
    for (const name of showNodes) {
      const node = m.getObjectByName(name);
      node?.traverse((o) => {
        const mesh = o as THREE.Mesh;
        if (mesh.isMesh) {
          const mat = (mesh.material as THREE.MeshStandardMaterial).clone();
          mat.emissive = new THREE.Color('#3d2a20');
          mat.emissiveIntensity = 0.5;
          mesh.material = mat;
        }
      });
    }
    if (externalWeapon) {
      markersRef.current = attachWeapon(m, externalWeapon);
    } else {
      // Trail markers on the first visible embedded weapon.
      for (const name of showNodes) {
        const node = m.getObjectByName(name);
        if (node) {
          const base = new THREE.Object3D();
          base.position.set(0, 0.05, 0);
          const tip = new THREE.Object3D();
          tip.position.set(0, embeddedTipY, 0);
          node.add(base, tip);
          markersRef.current = {base, tip};
          break;
        }
      }
    }
    return m;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gltf, manifest]);

  const mixer = useMemo(() => new THREE.AnimationMixer(model), [model]);
  const clipMap = useMemo(() => {
    const map = new Map<string, THREE.AnimationClip>();
    for (const c of gltf.animations) map.set(c.name, c);
    return map;
  }, [gltf]);
  const actionMap = useMemo(() => new Map<string, THREE.AnimationAction>(), [mixer]);

  const poseAt = (time: number) => {
    const g = groupRef.current;
    if (!g) return;
    const p = positionAt(track, time);
    g.position.set(p.x, 0, p.z);
    g.rotation.y = p.yaw;

    const actives = activeClipsAt(track.anims, time);
    const names = new Set(actives.map((a) => a.clip));
    for (const a of actives) {
      const clip = clipMap.get(a.clip);
      if (!clip) continue;
      let action = actionMap.get(a.clip);
      if (!action) {
        action = mixer.clipAction(clip);
        actionMap.set(a.clip, action);
      }
      if (!action.isRunning()) action.play();
      action.enabled = true;
      action.time = a.time;
      action.setEffectiveWeight(a.weight);
      action.setEffectiveTimeScale(1);
    }
    for (const [name, action] of actionMap) {
      if (!names.has(name) && action.isRunning()) action.stop();
    }
    mixer.update(0);
    g.updateMatrixWorld(true);
  };

  useFrame(() => {
    poseAt(t);
  });

  useImperativeHandle(ref, () => ({
    poseAt,
    weaponPointsAt: (time: number) => {
      poseAt(time);
      const mk = markersRef.current;
      if (!mk) return null;
      return {
        base: mk.base.getWorldPosition(new THREE.Vector3()),
        tip: mk.tip.getWorldPosition(new THREE.Vector3()),
      };
    },
    positionAt: (time: number) => {
      const p = positionAt(track, time);
      return new THREE.Vector3(p.x, 0, p.z);
    },
    clipLabel: (time: number) => activeClipNames(track.anims, time),
  }));

  return (
    <group ref={groupRef}>
      <primitive object={model} />
      {/* colored rim/back light so the character separates from the dark set */}
      <pointLight
        position={[0, 1.6, -0.9]}
        color={rimColor}
        intensity={3.4}
        distance={4}
        decay={2}
      />
      {/* soft front fill keeps faces readable in close-ups */}
      <pointLight
        position={[0, 1.4, 1.1]}
        color="#8fb8c9"
        intensity={0.9}
        distance={3}
        decay={2}
      />
    </group>
  );
};

export const KayKitCharacter = forwardRef(Inner);

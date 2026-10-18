import * as THREE from "three";
import { getGltf } from "../../assets";

export type WeaponSpec = {
  url: string;
  slotBone: string;
  tipY: number;
  // Local transform matching the pack's attachment convention
  // (as used by the embedded Knight weapons).
  position?: [number, number, number];
  quaternion?: [number, number, number, number];
  emissive?: string;
};

// Attaches a standalone KayKit weapon glTF under a handslot bone and
// returns the base/tip markers used by the sword trail.
export const attachWeapon = (
  model: THREE.Object3D,
  spec: WeaponSpec,
): { base: THREE.Object3D; tip: THREE.Object3D } | null => {
  const slot = model.getObjectByName(spec.slotBone);
  if (!slot) return null;
  const weapon = getGltf(spec.url).scene.clone(true);
  weapon.traverse((o) => {
    const mesh = o as THREE.Mesh;
    if (mesh.isMesh) {
      mesh.frustumCulled = false;
      // slight emissive so the blade reads at 640x360 in a dark scene
      const mat = (mesh.material as THREE.MeshStandardMaterial).clone();
      mat.emissive = new THREE.Color(spec.emissive ?? "#1d3a44");
      mat.emissiveIntensity = 0.7;
      mesh.material = mat;
    }
  });
  weapon.position.set(...(spec.position ?? [0, 0.0333, 0]));
  weapon.quaternion.set(...(spec.quaternion ?? [0, -1, 0, 0]));
  slot.add(weapon);

  const base = new THREE.Object3D();
  base.position.set(0, 0.05, 0);
  const tip = new THREE.Object3D();
  tip.position.set(0, spec.tipY, 0);
  weapon.add(base, tip);
  return { base, tip };
};

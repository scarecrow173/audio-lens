import React, {forwardRef} from 'react';
import {KAYKIT} from '../../data/kaykitManifest';
import {CharacterTrack} from '../../data/shots';
import {CharacterHandle, KayKitCharacter} from './KayKitCharacter';

// KAEL — Rogue body, light/fast swordsman, cyan identity.
// Weapon: standalone 1H sword attached to the right hand slot.
export const Kael = forwardRef<CharacterHandle, {track: CharacterTrack}>(
  ({track}, ref) => (
    <KayKitCharacter
      ref={ref}
      manifest={KAYKIT.rogue}
      track={track}
      showNodes={[]}
      externalWeapon={{url: 'weapons/sword_1handed.gltf', slotBone: 'handslot.r', tipY: 1.32}}
      rimColor="#38e8ff"
    />
  )
);

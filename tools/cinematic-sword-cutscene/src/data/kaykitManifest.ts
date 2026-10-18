// KayKit Character Pack: Adventurers (CC0) asset survey.
// Regenerate with: node scripts/inspect-kaykit.ts
// Source: https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0

export type ClipInfo = { name: string; duration: number };

export type CharacterManifest = {
  id: string;
  file: string;
  meshes: string[];
  bones: string[];
  // Mesh nodes parented to a handslot bone; visibility toggled per-loadout.
  attachments: { slot: string; nodes: string[] }[];
  clips: ClipInfo[];
};

const CLIP_DURATIONS: ClipInfo[] = [
  { name: "1H_Melee_Attack_Chop", duration: 1.067 },
  { name: "1H_Melee_Attack_Slice_Diagonal", duration: 1.0 },
  { name: "1H_Melee_Attack_Slice_Horizontal", duration: 1.067 },
  { name: "1H_Melee_Attack_Stab", duration: 1.6 },
  { name: "1H_Ranged_Aiming", duration: 1.067 },
  { name: "1H_Ranged_Reload", duration: 1.167 },
  { name: "1H_Ranged_Shoot", duration: 1.067 },
  { name: "1H_Ranged_Shooting", duration: 1.6 },
  { name: "2H_Melee_Attack_Chop", duration: 1.633 },
  { name: "2H_Melee_Attack_Slice", duration: 1.1 },
  { name: "2H_Melee_Attack_Spin", duration: 2.4 },
  { name: "2H_Melee_Attack_Spinning", duration: 0.667 },
  { name: "2H_Melee_Attack_Stab", duration: 1.6 },
  { name: "2H_Melee_Idle", duration: 1.067 },
  { name: "2H_Ranged_Aiming", duration: 1.6 },
  { name: "2H_Ranged_Reload", duration: 1.6 },
  { name: "2H_Ranged_Shoot", duration: 1.067 },
  { name: "2H_Ranged_Shooting", duration: 1.067 },
  { name: "Block", duration: 1.067 },
  { name: "Block_Attack", duration: 1.067 },
  { name: "Block_Hit", duration: 1.067 },
  { name: "Blocking", duration: 1.067 },
  { name: "Cheer", duration: 1.667 },
  { name: "Death_A", duration: 0.8 },
  { name: "Death_A_Pose", duration: 0.0 },
  { name: "Death_B", duration: 2.633 },
  { name: "Death_B_Pose", duration: 0.0 },
  { name: "Dodge_Backward", duration: 0.4 },
  { name: "Dodge_Forward", duration: 0.4 },
  { name: "Dodge_Left", duration: 0.4 },
  { name: "Dodge_Right", duration: 0.4 },
  { name: "Dualwield_Melee_Attack_Chop", duration: 1.267 },
  { name: "Dualwield_Melee_Attack_Slice", duration: 1.167 },
  { name: "Dualwield_Melee_Attack_Stab", duration: 1.6 },
  { name: "Hit_A", duration: 0.667 },
  { name: "Hit_B", duration: 0.867 },
  { name: "Idle", duration: 1.067 },
  { name: "Interact", duration: 1.3 },
  { name: "Jump_Full_Long", duration: 2.333 },
  { name: "Jump_Full_Short", duration: 1.167 },
  { name: "Jump_Idle", duration: 1.067 },
  { name: "Jump_Land", duration: 0.667 },
  { name: "Jump_Start", duration: 0.6 },
  { name: "Lie_Down", duration: 3.0 },
  { name: "Lie_Idle", duration: 2.667 },
  { name: "Lie_Pose", duration: 0.0 },
  { name: "Lie_StandUp", duration: 2.333 },
  { name: "PickUp", duration: 1.3 },
  { name: "Running_A", duration: 0.8 },
  { name: "Running_B", duration: 1.067 },
  { name: "Running_Strafe_Left", duration: 0.8 },
  { name: "Running_Strafe_Right", duration: 0.8 },
  { name: "Sit_Chair_Down", duration: 0.8 },
  { name: "Sit_Chair_Idle", duration: 3.6 },
  { name: "Sit_Chair_Pose", duration: 0.0 },
  { name: "Sit_Chair_StandUp", duration: 0.8 },
  { name: "Sit_Floor_Down", duration: 1.0 },
  { name: "Sit_Floor_Idle", duration: 4.0 },
  { name: "Sit_Floor_Pose", duration: 0.0 },
  { name: "Sit_Floor_StandUp", duration: 1.133 },
  { name: "Spellcast_Long", duration: 2.533 },
  { name: "Spellcast_Raise", duration: 2.1 },
  { name: "Spellcast_Shoot", duration: 0.933 },
  { name: "Spellcasting", duration: 0.667 },
  { name: "T-Pose", duration: 0.0 },
  { name: "Throw", duration: 1.367 },
  { name: "Unarmed_Idle", duration: 1.067 },
  { name: "Unarmed_Melee_Attack_Kick", duration: 0.933 },
  { name: "Unarmed_Melee_Attack_Punch_A", duration: 1.467 },
  { name: "Unarmed_Melee_Attack_Punch_B", duration: 1.667 },
  { name: "Unarmed_Pose", duration: 0.0 },
  { name: "Use_Item", duration: 1.6 },
  { name: "Walking_A", duration: 1.067 },
  { name: "Walking_B", duration: 1.067 },
  { name: "Walking_Backwards", duration: 1.067 },
  { name: "Walking_C", duration: 1.6 },
];

const BONES = [
  "root",
  "hips",
  "spine",
  "chest",
  "head",
  "upperarm.l",
  "lowerarm.l",
  "wrist.l",
  "hand.l",
  "handslot.l",
  "upperarm.r",
  "lowerarm.r",
  "wrist.r",
  "hand.r",
  "handslot.r",
  "upperleg.l",
  "lowerleg.l",
  "foot.l",
  "toes.l",
  "upperleg.r",
  "lowerleg.r",
  "foot.r",
  "toes.r",
  // IK helper bones exist in the rig but are not used by this demo.
  "kneeIK.l",
  "heelIK.l",
  "IK-foot.l",
  "control-foot-roll.l",
  "IK-toe.l",
  "control-heel-roll.l",
  "control-toe-roll.l",
  "kneeIK.r",
  "heelIK.r",
  "IK-foot.r",
  "control-foot-roll.r",
  "IK-toe.r",
  "control-heel-roll.r",
  "control-toe-roll.r",
  "elbowIK.l",
  "handIK.l",
  "elbowIK.r",
  "handIK.r",
];

export const KAYKIT: Record<"rogue" | "knight", CharacterManifest> = {
  rogue: {
    id: "rogue",
    file: "models/Rogue.glb",
    meshes: [
      "Rogue_ArmLeft",
      "Rogue_ArmRight",
      "Rogue_Body",
      "Rogue_Head",
      "Rogue_LegLeft",
      "Rogue_LegRight",
      "Rogue_Cape",
    ],
    bones: BONES,
    attachments: [
      { slot: "handslot.r", nodes: ["1H_Crossbow", "2H_Crossbow", "Knife", "Throwable"] },
      { slot: "handslot.l", nodes: ["Knife_Offhand"] },
    ],
    clips: CLIP_DURATIONS,
  },
  knight: {
    id: "knight",
    file: "models/Knight.glb",
    meshes: [
      "Knight_ArmLeft",
      "Knight_ArmRight",
      "Knight_Body",
      "Knight_Head",
      "Knight_LegLeft",
      "Knight_LegRight",
      "Knight_Cape",
      "Knight_Helmet",
    ],
    bones: BONES,
    attachments: [
      { slot: "handslot.r", nodes: ["1H_Sword", "2H_Sword"] },
      {
        slot: "handslot.l",
        nodes: [
          "1H_Sword_Offhand",
          "Badge_Shield",
          "Rectangle_Shield",
          "Round_Shield",
          "Spike_Shield",
        ],
      },
    ],
    clips: CLIP_DURATIONS,
  },
};

export const clipDuration = (name: string): number => {
  const c = CLIP_DURATIONS.find((x) => x.name === name);
  if (!c) throw new Error(`Unknown KayKit clip: ${name}`);
  return c.duration;
};

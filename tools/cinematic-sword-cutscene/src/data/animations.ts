// Semantic animation categories -> actual KayKit clip names.
// Clip names verified against the GLBs (see kaykitManifest.ts).

export const ANIM = {
  idle: "Idle",
  idle2h: "2H_Melee_Idle",
  unarmedIdle: "Unarmed_Idle",
  walk: "Walking_A",
  walkBack: "Walking_Backwards",
  run: "Running_A",
  attackSliceDiag: "1H_Melee_Attack_Slice_Diagonal",
  attackSliceHoriz: "1H_Melee_Attack_Slice_Horizontal",
  attackChop: "1H_Melee_Attack_Chop",
  attackStab: "1H_Melee_Attack_Stab",
  attack2hSlice: "2H_Melee_Attack_Slice",
  attack2hChop: "2H_Melee_Attack_Chop",
  block: "Block",
  blocking: "Blocking",
  blockHit: "Block_Hit",
  dodgeBack: "Dodge_Backward",
  dodgeRight: "Dodge_Right",
  hitA: "Hit_A",
  hitB: "Hit_B",
  land: "Jump_Land",
  raise: "Spellcast_Raise",
  powerUp: "Spellcast_Long",
  interact: "Interact",
} as const;

export type AnimKey = keyof typeof ANIM;

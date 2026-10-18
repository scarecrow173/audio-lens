// Dialogue is rendered inside GameHUD (see hud/GameHUD.tsx) so subtitle and
// HP state share one DOM overlay pass. This module re-exports the timing
// helper for clarity.
export { dialogueAt } from "../data/dialogue";

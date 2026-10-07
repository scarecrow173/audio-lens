import {Timeline} from './shots';

export type ActiveDialogue = {speaker: 'KAEL' | 'VAREN'; text: string};

// Dialogue is attached to shots; shown slightly inset from the cut.
export const dialogueAt = (tl: Timeline, t: number): ActiveDialogue | null => {
  const shot = tl.shots.find((s) => t >= s.start && t < s.end);
  if (!shot?.dialogue) return null;
  const showFrom = shot.start + 0.25;
  const showTo = shot.end - 0.2;
  if (t < showFrom || t >= showTo) return null;
  return shot.dialogue;
};

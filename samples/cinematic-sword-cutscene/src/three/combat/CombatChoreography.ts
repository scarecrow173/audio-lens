import {CharacterTrack} from '../../data/shots';
import {isAttackingAt} from '../characters/CharacterAnimationController';

// Whether a character's blade should currently draw a trail.
export const trailActive = (track: CharacterTrack, t: number): boolean =>
  isAttackingAt(track.anims, t);

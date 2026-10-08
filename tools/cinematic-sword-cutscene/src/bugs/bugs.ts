// Non-destructive video bug injection.
//
// Two bug classes:
//  - "remap": the output frame samples a different source frame. The whole
//    scene (camera, characters, particles, HUD) is a pure function of the
//    source frame, so remapping is bit-exact outside the affected range.
//  - "post": the source frame is rendered unchanged and a pixel overlay is
//    applied on top (black frames, brightness shifts, flashes).

export type BugProfile =
  | {type: 'none'}
  | {type: 'freeze'; startFrame: number; endFrame: number}
  | {type: 'blackFrame'; frames: readonly number[]}
  | {type: 'blackFrame'; startFrame: number; length: number}
  | {type: 'brightness'; kind: 'drop' | 'spike'; startFrame: number; endFrame: number; opacity: number}
  | {type: 'droppedFrames'; startFrame: number; endFrame: number; period: number; drop: number}
  | {type: 'stutter'; startFrame: number; endFrame: number}
  | {type: 'sceneShift'; atFrame: number; deltaFrames: number}
  | {type: 'missingScene'; startFrame: number; endFrame: number}
  | {type: 'corruptedTransition'; atFrame: number; flashFrames: number; blackFrames: number};

export const NO_BUG: BugProfile = {type: 'none'};

// Map output frame -> source (master timeline) frame.
export const mapSourceFrame = (outFrame: number, bug: BugProfile): number => {
  switch (bug.type) {
    case 'freeze':
      return outFrame >= bug.startFrame && outFrame < bug.endFrame
        ? bug.startFrame
        : outFrame;
    case 'droppedFrames': {
      const {startFrame: s, endFrame: e, period, drop} = bug;
      if (outFrame < s || outFrame >= e) return outFrame;
      const pos = (outFrame - s) % period;
      return pos >= period - drop ? outFrame - (pos - (period - drop)) : outFrame;
    }
    case 'stutter': {
      const {startFrame: s, endFrame: e} = bug;
      if (outFrame < s || outFrame >= e) return outFrame;
      // A B A B A B ... alternates between two adjacent source frames.
      return s + ((outFrame - s) % 2);
    }
    case 'sceneShift': {
      const {atFrame, deltaFrames: d} = bug;
      if (d > 0) {
        // Cut happens d frames later: hold the pre-cut frame, then jump.
        if (outFrame < atFrame) return outFrame;
        if (outFrame < atFrame + d) return atFrame - 1;
        return outFrame;
      }
      // Cut happens earlier: skip ahead after the original boundary.
      return outFrame >= atFrame + d && outFrame < atFrame
        ? outFrame - d
        : outFrame;
    }
    case 'missingScene': {
      const {startFrame: s, endFrame: e} = bug;
      return outFrame >= s ? outFrame + (e - s) : outFrame;
    }
    default:
      return outFrame;
  }
};

// Post-process overlays evaluated on the *output* frame.
export type PostOverlay =
  | {kind: 'black'}
  | {kind: 'dim'; opacity: number}
  | {kind: 'white'; opacity: number};

export const postOverlaysAt = (outFrame: number, bug: BugProfile): PostOverlay[] => {
  switch (bug.type) {
    case 'blackFrame': {
      const hit =
        'frames' in bug
          ? bug.frames.includes(outFrame)
          : outFrame >= bug.startFrame && outFrame < bug.startFrame + bug.length;
      return hit ? [{kind: 'black'}] : [];
    }
    case 'brightness':
      if (outFrame >= bug.startFrame && outFrame < bug.endFrame) {
        return bug.kind === 'drop'
          ? [{kind: 'dim', opacity: bug.opacity}]
          : [{kind: 'white', opacity: bug.opacity}];
      }
      return [];
    case 'corruptedTransition': {
      const {atFrame, flashFrames, blackFrames} = bug;
      if (outFrame >= atFrame && outFrame < atFrame + flashFrames) {
        return [{kind: 'white', opacity: 0.9}];
      }
      if (
        outFrame >= atFrame + flashFrames &&
        outFrame < atFrame + flashFrames + blackFrames
      ) {
        return [{kind: 'black'}];
      }
      return [];
    }
    default:
      return [];
  }
};

// Machine-readable metadata for audio-lens verification fixtures.
export const bugMetadata = (bug: BugProfile): Record<string, unknown> => {
  const {type, ...rest} = bug as Record<string, unknown>;
  return {type, ...rest};
};

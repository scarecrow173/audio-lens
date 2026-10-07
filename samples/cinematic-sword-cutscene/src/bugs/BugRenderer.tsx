import React from 'react';
import {AbsoluteFill, useCurrentFrame} from 'remotion';
import {BugProfile, mapSourceFrame, postOverlaysAt} from './bugs';
import {RenderFrameProvider} from './RenderFrameContext';

// Applies a BugProfile around the unchanged game-capture renderer:
// source frames are remapped for the scene, and post overlays are painted
// on top of the DOM (so e.g. a black frame also covers the HUD).
export const BugRenderer: React.FC<{
  bug: BugProfile;
  children: React.ReactNode;
}> = ({bug, children}) => {
  const outFrame = useCurrentFrame();
  const src = mapSourceFrame(outFrame, bug);
  const overlays = postOverlaysAt(outFrame, bug);

  return (
    <RenderFrameProvider frame={src}>
      {children}
      {overlays.map((o, i) => (
        <AbsoluteFill
          key={i}
          style={{
            background:
              o.kind === 'black'
                ? '#000'
                : o.kind === 'dim'
                  ? `rgba(0,0,0,${o.opacity})`
                  : `rgba(255,255,255,${o.opacity})`,
          }}
        />
      ))}
    </RenderFrameProvider>
  );
};

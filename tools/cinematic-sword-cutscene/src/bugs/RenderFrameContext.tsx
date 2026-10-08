import React, {createContext, useContext} from 'react';
import {useCurrentFrame} from 'remotion';

// The source (master-timeline) frame currently being produced.
// Equals the output frame for the normal render; differs under
// time-remap bug profiles.
const RenderFrameContext = createContext<number | null>(null);

export const RenderFrameProvider: React.FC<{
  frame: number;
  children: React.ReactNode;
}> = ({frame, children}) => (
  <RenderFrameContext.Provider value={frame}>
    {children}
  </RenderFrameContext.Provider>
);

export const useRemappedFrame = (): number => {
  const ctx = useContext(RenderFrameContext);
  const frame = useCurrentFrame();
  return ctx === null ? frame : ctx;
};

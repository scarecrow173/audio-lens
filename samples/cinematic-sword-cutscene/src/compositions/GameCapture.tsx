import React, {useEffect, useMemo, useRef, useState} from 'react';
import {
  AbsoluteFill,
  continueRender,
  delayRender,
  useVideoConfig,
} from 'remotion';
import {useFrame, useThree} from '@react-three/fiber';
import {ThreeCanvas} from '@remotion/three';
import {preloadAssets} from '../assets';
import {MASTER_TIMELINE, TEST_TIMELINE} from '../data/masterTimeline';
import {BugProfile, NO_BUG} from '../bugs/bugs';
import {BugRenderer} from '../bugs/BugRenderer';
import {GameScene, createSceneHandles} from '../three/GameScene';
import {GameHUD} from '../hud/GameHUD';
import {DebugOverlay} from '../hud/DebugOverlay';

// Assets are fully parsed before the scene mounts so nothing suspends
// mid-frame; captures stay deterministic.
const useReady = () => {
  const [ready, setReady] = useState(false);
  const [handle] = useState(() => delayRender('preload-assets'));
  useEffect(() => {
    preloadAssets()
      .then(() => setReady(true))
      .catch((e) => console.error(e))
      .finally(() => continueRender(handle));
  }, [handle]);
  return ready;
};

// Keeps a delayRender handle open until the WebGL canvas has produced at
// least one frame — Remotion then captures a painted canvas, never an
// empty one.
const FirstFrameSignal: React.FC<{handle: number}> = ({handle}) => {
  const done = useRef(false);
  const scene = useThree((s) => s.scene);
  useFrame(() => {
    if (!done.current && scene.children.length > 0) {
      done.current = true;
      continueRender(handle);
    }
  });
  return null;
};

// ThreeCanvas renders via advance() on frame changes only (frameloop is
// Remotion-driven), so a scene that mounts late must advance() once itself.
const KickRender: React.FC = () => {
  const advance = useThree((s) => s.advance);
  useEffect(() => {
    advance(performance.now());
  }, [advance]);
  return null;
};

export type GameCaptureProps = {
  timelineId: 'master' | 'test';
  bug: BugProfile;
  debug: boolean;
};

export const GameCapture: React.FC<GameCaptureProps> = ({
  timelineId = 'master',
  bug = NO_BUG,
  debug = false,
}) => {
  const ready = useReady();
  const [firstFrame] = useState(() => delayRender('first-webgl-frame'));
  const {width, height} = useVideoConfig();
  const timeline = timelineId === 'test' ? TEST_TIMELINE : MASTER_TIMELINE;
  const handles = useMemo(createSceneHandles, []);

  return (
    <BugRenderer bug={bug}>
      <AbsoluteFill style={{background: '#000'}}>
        <ThreeCanvas
          width={width}
          height={height}
          gl={{antialias: true, alpha: false, preserveDrawingBuffer: true}}
          camera={{fov: 42, position: [0, 2, 6], near: 0.1, far: 80}}
          style={{position: 'absolute', inset: 0}}
        >
          {ready ? (
            <>
              <GameScene timeline={timeline} handles={handles} />
              <FirstFrameSignal handle={firstFrame} />
              <KickRender />
            </>
          ) : null}
        </ThreeCanvas>
        <GameHUD timeline={timeline} />
        {debug ? <DebugOverlay timeline={timeline} bug={bug} /> : null}
      </AbsoluteFill>
    </BugRenderer>
  );
};

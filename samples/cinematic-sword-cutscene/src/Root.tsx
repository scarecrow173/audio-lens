import React from 'react';
import {Composition} from 'remotion';
import {GameCapture, GameCaptureProps} from './compositions/GameCapture';
import {NO_BUG} from './bugs/bugs';

const W = 640;
const H = 360;
const FPS = 30;

export const Root: React.FC = () => (
  <>
    <Composition
      id="GameCaptureTest"
      component={GameCapture}
      width={W}
      height={H}
      fps={FPS}
      durationInFrames={300}
      defaultProps={
        {
          timelineId: 'test',
          bug: NO_BUG,
          debug: false,
        } satisfies GameCaptureProps
      }
    />
    <Composition
      id="GameCaptureNormal"
      component={GameCapture}
      width={W}
      height={H}
      fps={FPS}
      durationInFrames={1500}
      defaultProps={
        {
          timelineId: 'master',
          bug: NO_BUG,
          debug: false,
        } satisfies GameCaptureProps
      }
    />
    {/* Generic bug renderer: override `bug` via --props to preview any profile. */}
    <Composition
      id="GameCaptureBugPreview"
      component={GameCapture}
      width={W}
      height={H}
      fps={FPS}
      durationInFrames={1500}
      defaultProps={
        {
          timelineId: 'master',
          bug: NO_BUG,
          debug: true,
        } satisfies GameCaptureProps
      }
    />
    <Composition
      id="GameCaptureBugFreeze"
      component={GameCapture}
      width={W}
      height={H}
      fps={FPS}
      durationInFrames={1500}
      defaultProps={
        {
          timelineId: 'master',
          // freeze 32.0s - 33.0s (mid sword-combo)
          bug: {type: 'freeze', startFrame: 960, endFrame: 990},
          debug: false,
        } satisfies GameCaptureProps
      }
    />
  </>
);

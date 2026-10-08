import React, {useRef} from 'react';
import * as THREE from 'three';
import {Timeline} from '../data/shots';
import {Environment} from './environment/Environment';
import {SceneLighting} from './lighting/SceneLighting';
import {GameCamera} from './camera/GameCamera';
import {Kael} from './characters/Kael';
import {Varen} from './characters/Varen';
import {CharacterHandle} from './characters/KayKitCharacter';
import {SwordTrail} from './combat/SwordTrail';
import {Sparks} from './vfx/Sparks';
import {ImpactParticles} from './vfx/ImpactParticles';
import {AmbientDust} from './vfx/AmbientDust';
import {FogParticles} from './vfx/FogParticles';
import {PowerParticles} from './vfx/PowerParticles';

export type SceneHandles = {
  kael: React.RefObject<CharacterHandle | null>;
  varen: React.RefObject<CharacterHandle | null>;
};

export const GameScene: React.FC<{
  timeline: Timeline;
  handles: SceneHandles;
}> = ({timeline, handles}) => {
  return (
    <>
      <color attach="background" args={['#0e1622']} />
      <fogExp2 attach="fog" args={['#13202e', 0.034]} />
      <SceneLighting timeline={timeline} />
      <Environment />
      {/* trails subscribe before characters so they sample past poses first;
          each restores the current pose afterwards */}
      <SwordTrail character={handles.kael} track={timeline.kael} color="#67f2ff" />
      <SwordTrail character={handles.varen} track={timeline.varen} color="#ff9a5a" />
      <Kael ref={handles.kael} track={timeline.kael} />
      <Varen ref={handles.varen} track={timeline.varen} />
      <AmbientDust />
      <FogParticles />
      <Sparks impacts={timeline.impacts} />
      <ImpactParticles timeline={timeline} />
      <PowerParticles window={timeline.powerUp} track={timeline.kael} />
      <GameCamera timeline={timeline} />
    </>
  );
};

export const createSceneHandles = (): SceneHandles => ({
  kael: React.createRef<CharacterHandle>(),
  varen: React.createRef<CharacterHandle>(),
});

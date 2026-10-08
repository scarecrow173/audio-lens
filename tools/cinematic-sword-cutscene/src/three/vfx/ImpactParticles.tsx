import React from 'react';
import {ImpactEvent, Timeline} from '../../data/shots';
import {ParticleBurst} from './ParticleSystem';

// Dust/smoke puff + floor debris chunks on heavy hits, landings, knockbacks.
export const ImpactParticles: React.FC<{timeline: Timeline}> = ({timeline}) => {
  const puffEvents = [
    ...timeline.impacts.map((i) => ({t: i.t, pos: i.pos})),
    ...timeline.dustBursts.map((d) => ({t: d.t, pos: d.pos})),
  ];
  return (
    <group>
      <ParticleBurst
        events={puffEvents}
        params={{
          count: 40,
          seed: 8107,
          color: '#7d8fa3',
          speedMin: 0.4,
          speedMax: 1.8,
          flatness: 0.6,
          gravity: -0.6,
          life: 0.9,
          size: 0.22,
          fadePower: 1.4,
          upBias: 0.25,
        }}
      />
      {/* debris chunks */}
      <ParticleBurst
        events={timeline.debrisBursts.map((d) => ({t: d.t, pos: d.pos}))}
        params={{
          count: 26,
          seed: 9109,
          color: '#4a5663',
          speedMin: 1.2,
          speedMax: 3.6,
          flatness: 0.3,
          gravity: 11,
          life: 0.8,
          size: 0.09,
          fadePower: 1.1,
          upBias: 0.5,
        }}
      />
    </group>
  );
};

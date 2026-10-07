import React from 'react';
import {ImpactEvent} from '../../data/shots';
import {ParticleBurst} from './ParticleSystem';

// Bright metal-clash sparks at each sword impact.
export const Sparks: React.FC<{impacts: readonly ImpactEvent[]}> = ({impacts}) => (
  <ParticleBurst
    events={impacts.map((i) => ({t: i.t, pos: i.pos}))}
    params={{
      count: 70,
      seed: 6121,
      color: '#ffd77a',
      speedMin: 1.5,
      speedMax: 5.5,
      flatness: 0.1,
      gravity: 9.5,
      life: 0.55,
      size: 0.045,
      fadePower: 1.8,
      upBias: 0.15,
    }}
  />
);

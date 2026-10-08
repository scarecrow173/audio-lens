import React from 'react';
import {Floor} from './Floor';
import {Pillars} from './Pillars';
import {Ruins} from './Ruins';

export const Environment: React.FC = () => (
  <group>
    <Floor />
    <Pillars />
    <Ruins />
  </group>
);

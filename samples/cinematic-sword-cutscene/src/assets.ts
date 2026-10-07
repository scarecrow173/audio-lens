import * as THREE from 'three';
import {GLTFLoader, GLTF} from 'three/examples/jsm/loaders/GLTFLoader.js';
import {staticFile} from 'remotion';

// Synchronous GLTF store. Assets are fully parsed BEFORE the scene mounts
// (gated by delayRender), so components never suspend — which keeps
// Remotion frame captures deterministic.

const ASSETS = [
  'models/Rogue.glb',
  'models/Knight.glb',
  'weapons/sword_1handed.gltf',
];

const cache = new Map<string, GLTF>();
let loadPromise: Promise<void> | null = null;

export const preloadAssets = (): Promise<void> => {
  if (!loadPromise) {
    THREE.Cache.enabled = true;
    const loader = new GLTFLoader();
    loadPromise = Promise.all(
      ASSETS.map(
        (u) =>
          new Promise<void>((resolve, reject) =>
            loader.load(
              staticFile(u),
              (g) => {
                cache.set(u, g);
                resolve();
              },
              undefined,
              (e) => reject(new Error(`failed to load ${u}: ${e}`))
            )
          )
      )
    ).then(() => undefined);
  }
  return loadPromise;
};

export const getGltf = (url: string): GLTF => {
  const g = cache.get(url);
  if (!g) throw new Error(`asset not preloaded: ${url}`);
  return g;
};

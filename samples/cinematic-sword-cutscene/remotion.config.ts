import {Config} from '@remotion/cli/config';

Config.setVideoImageFormat('jpeg');
Config.setOverwriteOutput(true);
// Software WebGL is deprecated by Chromium unless --enable-unsafe-swiftshader
// is passed; use ANGLE (falls back to GPU) — see out/ determinism notes.
Config.setChromiumOpenGlRenderer('angle');
Config.setEntryPoint('src/index.ts');
// Models, weapons and generated audio all live under assets/.
Config.setPublicDir('assets');

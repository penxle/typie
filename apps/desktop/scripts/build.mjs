#!/usr/bin/env node

import { build } from 'vite';

const mode = process.env.MODE ?? 'prod';

for (const configFile of ['vite.main.config.ts', 'vite.preload.config.ts', 'vite.renderer.config.ts']) {
  await build({ configFile, mode });
}

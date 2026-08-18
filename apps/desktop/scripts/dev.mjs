#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { build, createServer } from 'vite';

const rendererServer = await createServer({ configFile: 'vite.renderer.config.ts' });
await rendererServer.listen();

let electron = null;

const startElectron = () => {
  electron?.kill();
  const proc = spawn(process.platform === 'win32' ? 'npx.cmd' : 'npx', ['electron', '.'], {
    stdio: 'inherit',
    env: { ...process.env, ELECTRON_RENDERER_URL: 'http://localhost:5300', ENVIRONMENT: process.env.ENVIRONMENT ?? 'local' },
  });
  electron = proc;
  proc.on('exit', (code) => {
    if (electron !== proc) return;
    rendererServer.close().then(() => process.exit(code ?? 0));
  });
};

let mainReady = false;
let preloadReady = false;

const watch = async (configFile, onDone) => {
  const watcher = await build({ configFile, build: { watch: {} }, mode: 'development' });
  watcher.on('event', (event) => {
    if (event.code === 'END') onDone();
  });
};

const restart = () => {
  if (mainReady && preloadReady) startElectron();
};

await watch('vite.preload.config.ts', () => {
  preloadReady = true;
  restart();
});
await watch('vite.main.config.ts', () => {
  mainReady = true;
  restart();
});

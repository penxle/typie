/// <reference types="vitest/config" />

import path from 'node:path';
import { fileURLToPath, URL } from 'node:url';
import { sveltekit } from '@sveltejs/kit/vite';
import { svg } from '@typie/lib/vite';
import mearie from 'mearie/vite';
import { FileSystemIconLoader } from 'unplugin-icons/loaders';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';
import type { Plugin, UserConfig } from 'vite';

const currentDir = fileURLToPath(new URL('.', import.meta.url));
const editorPkgDir = path.resolve(currentDir, '../../crates/editor/pkg');

const wasmReloadPlugin = (): Plugin => {
  let timer: ReturnType<typeof setTimeout>;
  const changedFiles = new Set<string>();

  return {
    name: 'wasm-reload',
    configureServer(server) {
      server.watcher.add([editorPkgDir]);
    },
    handleHotUpdate({ file, server }) {
      if (!file.startsWith(editorPkgDir) || file.endsWith('.gitignore')) {
        return;
      }

      changedFiles.add(file);
      clearTimeout(timer);
      timer = setTimeout(() => {
        const time = new Date().toLocaleTimeString();
        const filesArray = [...changedFiles];
        const mainFile = path.basename(filesArray[0]);
        const extraCount = filesArray.length - 1;
        const fileInfo = extraCount > 0 ? `${mainFile} (+${extraCount})` : mainFile;

        console.log(
          `\u{1B}[90m${time}\u{1B}[0m \u{1B}[36m[wasm-reload]\u{1B}[0m \u{1B}[32mWASM Reloaded\u{1B}[0m \u{1B}[90m${fileInfo}\u{1B}[0m`,
        );
        server.ws.send({
          type: 'full-reload',
          path: '*',
        });
        changedFiles.clear();
      }, 100);
      return [];
    },
  };
};

const config = {
  clearScreen: false,
  plugins: [
    svg(),
    icons({
      scale: 1,
      compiler: 'svelte',
      customCollections: {
        typie: FileSystemIconLoader('./src/icons'),
      },
    }),
    mearie(),
    sveltekit(),
    wasmReloadPlugin(),
  ],
  optimizeDeps: {
    exclude: ['@typie/editor', '@typie/editor-ffi'],
  },
  server: {
    port: 4000,
    strictPort: true,
    fs: {
      allow: ['../..'],
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
};

export default defineConfig(config as UserConfig);

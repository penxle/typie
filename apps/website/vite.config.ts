/// <reference types="vitest/config" />

import path from 'node:path';
import { fileURLToPath, URL } from 'node:url';
import { sveltekit } from '@sveltejs/kit/vite';
import { svg } from '@typie/lib/vite';
import mearie from 'mearie/vite';
import { FileSystemIconLoader } from 'unplugin-icons/loaders';
import icons from 'unplugin-icons/vite';
import { defaultClientConditions, defineConfig } from 'vite';
import type { ConfigEnv, Plugin, UserConfig } from 'vite';

const currentDir = fileURLToPath(new URL('.', import.meta.url));
const editorPkgDir = path.resolve(currentDir, '../../crates/editor/pkg');
const prismPkgDir = path.resolve(currentDir, '../../crates/prism-ui-web/pkg');

const wasmReloadPlugin = (): Plugin => {
  let timer: ReturnType<typeof setTimeout>;
  const changedFiles = new Set<string>();

  return {
    name: 'wasm-reload',
    configureServer(server) {
      server.watcher.add([editorPkgDir, prismPkgDir]);
    },
    handleHotUpdate({ file, server }) {
      if ((!file.startsWith(editorPkgDir) && !file.startsWith(prismPkgDir)) || file.endsWith('.gitignore')) {
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

export const createConfig = ({ mode }: Pick<ConfigEnv, 'mode'>) => ({
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
    exclude: ['@typie/editor-ffi', '@typie/prism-ui', '@typie/prism-ui-web'],
  },
  ...(mode === 'test' && { resolve: { conditions: [...defaultClientConditions] } }),
  server: {
    port: 4000,
    strictPort: true,
    fs: {
      allow: ['../..'],
    },
  },
  test: {
    environment: 'jsdom',
    exclude: ['src/**/*.browser.test.ts'],
    include: ['src/**/*.test.ts'],
    setupFiles: ['./src/vitest-setup.ts'],
  },
});

export default defineConfig((env) => createConfig(env) as UserConfig);

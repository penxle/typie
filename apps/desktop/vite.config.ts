import { sveltekit } from '@sveltejs/kit/vite';
import { svg } from '@typie/lib/vite';
import { sark } from '@typie/sark/vite';
import { FileSystemIconLoader } from 'unplugin-icons/loaders';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
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
    sark(),
    sveltekit(),
  ],
  server: {
    port: 5000,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 5001 } : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});

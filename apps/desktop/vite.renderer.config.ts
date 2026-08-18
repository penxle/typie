import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svg } from '@typie/lib/vite';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  root: 'src/renderer',
  base: './',
  envPrefix: 'PUBLIC_',
  plugins: [svg(), icons({ scale: 1, compiler: 'svelte' }), svelte()],
  build: {
    outDir: '../../dist/renderer',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        chrome: 'chrome/index.html',
        login: 'login/index.html',
        offline: 'offline/index.html',
        crash: 'crash/index.html',
      },
    },
  },
  server: { port: 5300, strictPort: true },
});

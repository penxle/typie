import { svg } from '@glitter/lib/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    svg(),
    icons({
      scale: 1,
      compiler: 'svelte',
    }),
    sveltekit(),
  ],
  server: {
    port: 4000,
    strictPort: true,
  },
});

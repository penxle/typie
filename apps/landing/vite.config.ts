import { sveltekit } from '@sveltejs/kit/vite';
import { svg } from '@typie/lib/vite';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  plugins: [svg(), icons({ scale: 1, compiler: 'svelte' }), sveltekit()],
  optimizeDeps: {
    exclude: ['@typie/prism-ui', '@typie/prism-ui-web'],
  },
  server: {
    port: 5300,
    strictPort: true,
    fs: { allow: ['../..'] },
  },
});

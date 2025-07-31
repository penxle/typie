import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  plugins: [sveltekit()],
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

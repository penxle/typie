import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
  compilerOptions: {
    warningFilter: (warning) => !warning.code.startsWith('state_referenced_locally'),
  },
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    alias: {
      '$assets/*': './src/assets/*',
    },
    paths: { relative: false },
  },
};

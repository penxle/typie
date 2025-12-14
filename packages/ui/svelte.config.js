import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
  compilerOptions: {
    warningFilter: (warning) => !warning.code.startsWith('state_referenced_locally'),
  },
  preprocess: vitePreprocess(),
  kit: {
    alias: {
      '@/*': '../../apps/api/src/*',
    },
  },
};

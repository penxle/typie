import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { node } from '@typie/adapter-node';

/** @type {import('@sveltejs/kit').Config} */
export default {
  compilerOptions: {
    warningFilter: (warning) => !warning.code.startsWith('state_referenced_locally'),
  },
  preprocess: vitePreprocess(),

  kit: {
    adapter: node(),
    alias: {
      '@/*': '../api/src/*',
      '$assets/*': './src/assets/*',
    },
    paths: { relative: false },
    csrf: { trustedOrigins: ['*'] },
    output: { preloadStrategy: 'preload-mjs' },
    typescript: {
      config: (config) => ({
        ...config,
        compilerOptions: {
          ...config.compilerOptions,
        },
        include: [...config.include, '../scripts/**/*.ts'],
      }),
    },
    version: { pollInterval: 60 * 1000 },
  },

  vitePlugin: {
    inspector: {
      toggleKeyCombo: 'alt-x',
      toggleButtonPos: 'bottom-right',
    },
  },
};

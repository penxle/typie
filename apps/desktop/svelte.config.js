import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: 'index.html',
    }),
    alias: {
      '@/*': '../api/src/*',
      '$assets/*': './src/assets/*',
      $graphql: './.sark',
    },
    paths: { relative: false },
    csrf: { checkOrigin: false },
    output: { preloadStrategy: 'preload-mjs' },
    typescript: {
      config: (config) => ({
        ...config,
        compilerOptions: {
          ...config.compilerOptions,
          rootDirs: [...config.compilerOptions.rootDirs, '../.sark/types'],
        },
      }),
    },
  },
};

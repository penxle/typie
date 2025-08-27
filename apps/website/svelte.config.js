import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { node } from '@typie/adapter-node';

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),

  kit: {
    adapter: node(),
    alias: {
      '@/*': '../api/src/*',
      '$assets/*': './src/assets/*',
      $graphql: './.sark',
    },
    paths: { relative: false },
    csrf: { trustedOrigins: ['*'] },
    output: { preloadStrategy: 'preload-mjs' },
    typescript: {
      config: (config) => ({
        ...config,
        compilerOptions: {
          ...config.compilerOptions,
          rootDirs: [...config.compilerOptions.rootDirs, '../.sark/types'],
        },
        include: [...config.include, '../pulumi/**/*.ts', '../scripts/**/*.ts'],
      }),
    },
    version: { pollInterval: 60 * 1000 },
  },
};

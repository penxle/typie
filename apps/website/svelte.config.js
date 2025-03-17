import { bun } from '@glitter/adapter-bun';
import { sveltePreprocess } from 'svelte-preprocess';

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: sveltePreprocess(),

  kit: {
    adapter: bun(),
    alias: {
      '@/*': '../api/src/*',
      '$assets/*': './src/assets/*',
      '$styled-system/*': './styled-system/*',
      $graphql: '.gql',
    },
    files: {
      hooks: {
        server: 'src/hooks/server',
        client: 'src/hooks/client',
        universal: 'src/hooks/universal',
      },
    },
    paths: { relative: false },
    output: { preloadStrategy: 'preload-mjs' },
    typescript: {
      config: (config) => ({
        ...config,
        include: [...config.include, '../pulumi/**/*.ts', '../scripts/**/*.ts'],
      }),
    },
    version: { pollInterval: 60 * 1000 },
  },
};

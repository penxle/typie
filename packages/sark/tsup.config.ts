import { defineConfig } from 'tsup';

export default defineConfig([
  {
    entry: {
      vite: 'src/codegen/vite/index.ts',
    },
    dts: true,
    format: ['esm'],
    external: [/^\$/],

    esbuildOptions: (options) => {
      options.packages = 'external';
    },
  },
]);

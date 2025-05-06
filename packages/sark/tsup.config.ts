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
  {
    entry: {
      cli: 'src/codegen/cli.ts',
    },
    format: ['esm'],
    external: [/^\$/],

    esbuildOptions: (options) => {
      options.packages = 'external';
    },
  },
]);

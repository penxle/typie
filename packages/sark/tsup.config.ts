import { defineConfig } from 'tsup';

export default defineConfig([
  {
    entry: {
      cli: 'src/codegen/cli.ts',
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

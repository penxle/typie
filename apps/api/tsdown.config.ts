import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/**/*.tsx'],
  format: 'esm',
  dts: true,

  clean: false,
  outDir: 'src',
  outExtensions: () => ({ js: '.tsx.js', dts: '.tsx.d.ts' }),

  unbundle: true,
  deps: { neverBundle: [/./] },
});

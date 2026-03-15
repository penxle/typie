import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: {
    handler: 'src/handler.ts',
  },

  format: 'cjs',
  outDir: 'dist/function',

  deps: { neverBundle: [/^@aws-sdk\//, 'sharp'] },
});

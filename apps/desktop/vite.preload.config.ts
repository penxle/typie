import { defineConfig } from 'vite';

export default defineConfig({
  ssr: { noExternal: true },
  build: {
    outDir: 'dist/preload',
    emptyOutDir: true,
    target: 'node22',
    ssr: true,
    minify: false,
    sourcemap: true,
    lib: {
      entry: { chrome: 'src/preload/chrome.ts', page: 'src/preload/page.ts', tab: 'src/preload/tab.ts' },
      formats: ['cjs'],
      fileName: (_format, name) => `${name}.cjs`,
    },
    rollupOptions: { external: ['electron'] },
  },
});

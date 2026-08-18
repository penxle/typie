import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite';

const pkg = JSON.parse(readFileSync(new URL('package.json', import.meta.url), 'utf8')) as { dependencies?: Record<string, string> };
const deps = Object.keys(pkg.dependencies ?? {});

export default defineConfig({
  envPrefix: 'PUBLIC_',
  ssr: { noExternal: true },
  build: {
    outDir: 'dist/main',
    emptyOutDir: true,
    target: 'node22',
    ssr: true,
    minify: false,
    sourcemap: true,
    lib: { entry: 'src/main/index.ts', formats: ['es'], fileName: () => 'index.js' },
    rollupOptions: { external: (id) => id === 'electron' || deps.some((dep) => id === dep || id.startsWith(`${dep}/`)) },
  },
});

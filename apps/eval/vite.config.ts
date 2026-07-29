/// <reference types="vitest/config" />

import { sveltekit } from '@sveltejs/kit/vite';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  plugins: [icons({ scale: 1, compiler: 'svelte' }), sveltekit()],
  server: {
    port: 5100,
    strictPort: true,
    fs: { allow: ['../..'] },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts', 'core/**/*.test.ts', 'generations/**/*.test.ts', 'flows/src/**/*.test.ts', 'scripts/**/*.test.ts'],
  },
});

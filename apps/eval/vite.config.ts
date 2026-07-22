/// <reference types="vitest/config" />

import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  plugins: [sveltekit()],
  server: {
    port: 5100,
    strictPort: true,
    fs: { allow: ['../..'] },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});

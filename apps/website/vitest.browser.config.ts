/// <reference types="vitest/config" />

import { playwright } from '@vitest/browser-playwright';
import { defaultClientConditions, defineConfig } from 'vite';
import { createConfig } from './vite.config.ts';
import type { UserConfig } from 'vite';

const base = createConfig({ mode: 'test' }) as UserConfig;

export default defineConfig({
  ...base,
  resolve: { ...base.resolve, conditions: [...defaultClientConditions] },
  test: {
    browser: {
      enabled: true,
      headless: true,
      provider: playwright({ contextOptions: { hasTouch: true } }),
      screenshotDirectory: '.vitest-screenshots',
      instances: [{ browser: 'chromium' }],
    },
    include: ['src/**/*.browser.test.ts'],
    setupFiles: ['./src/vitest-setup.ts'],
  },
});

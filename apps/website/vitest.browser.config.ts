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
      commands: {
        dismissDocumentReload: async ({ page }) => {
          let prompt: string | undefined;
          const dismiss = async (dialog: { type: () => string; dismiss: () => Promise<void> }) => {
            prompt = dialog.type();
            await dialog.dismiss();
          };
          page.on('dialog', dismiss);
          try {
            await page.reload({ timeout: 5000 }).catch((err: unknown) => {
              if (prompt !== 'beforeunload') throw err;
            });
            return prompt;
          } finally {
            page.off('dialog', dismiss);
          }
        },
      },
      screenshotDirectory: '.vitest-screenshots',
      instances: [{ browser: 'chromium' }],
    },
    include: ['src/**/*.browser.test.ts'],
    setupFiles: ['./src/vitest-setup.ts'],
  },
});

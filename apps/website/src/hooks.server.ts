import '$lib/polyfills';
import '@typie/lib/dayjs';

import { isAggregatedError } from '@mearie/svelte';
import * as Sentry from '@sentry/sveltekit';
import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/public';
import { PUBLIC_IMAGE_TAG } from '$env/static/public';
import type { Handle, HandleServerError } from '@sveltejs/kit';

Sentry.init({
  enabled: !dev,
  dsn: env.PUBLIC_SENTRY_DSN,
  environment: env.PUBLIC_ENVIRONMENT,
  release: PUBLIC_IMAGE_TAG,
  sendDefaultPii: true,
  enableLogs: true,
  tracesSampleRate: 0.1,
  integrations: (defaults) => defaults.filter((i) => i.name !== 'NodeSystemError'),
});

const log = logger.getChild('http');

const theme: Handle = async ({ event, resolve }) => {
  const theme = event.cookies.get('typie-th');
  const lightVariant = event.cookies.get('typie-th-lv') ?? 'white';
  const darkVariant = event.cookies.get('typie-th-dv') ?? 'black';

  return resolve(event, {
    transformPageChunk: ({ html }) => {
      if (event.url.pathname.includes('landing')) {
        return html.replace('%app.theme%', 'light').replace('%app.variant.light%', 'white').replace('%app.variant.dark%', 'black');
      }

      const defaultTheme = event.url.pathname.includes('_webview') ? 'light' : 'auto';
      const themeValue = theme && ['auto', 'light', 'dark'].includes(theme) ? theme : defaultTheme;
      return html
        .replace('%app.theme%', themeValue)
        .replace('%app.variant.light%', lightVariant)
        .replace('%app.variant.dark%', darkVariant);
    },
  });
};

const header: Handle = async ({ event, resolve }) => {
  return resolve(event, {
    filterSerializedResponseHeaders: (name) => {
      const n = name.toLowerCase();

      if (n === 'content-type') {
        return true;
      }

      return false;
    },
  });
};

const errorHandler: HandleServerError = ({ error, status, message }) => {
  if (isAggregatedError(error)) {
    log.error('Server error {*}', { status, message, errors: error.errors });
  } else {
    log.error('Server error {*}', { status, message, error });
  }
};

export const handle = sequence(Sentry.sentryHandle(), logging, theme, header);
export const handleError = Sentry.handleErrorWithSentry(errorHandler);

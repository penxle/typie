import '$lib/polyfills';
import '@typie/lib/dayjs';

import { isAggregatedError } from '@mearie/svelte';
import * as Sentry from '@sentry/sveltekit';
import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/public';
import { PUBLIC_IMAGE_TAG } from '$env/static/public';
import { resolveThemeAttributes } from '$lib/theme-ssr';
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
  const attrs = resolveThemeAttributes({
    routeId: event.route.id,
    pathname: event.url.pathname,
    cookies: {
      theme: event.cookies.get('typie-th'),
      light: event.cookies.get('typie-th-lv'),
      dark: event.cookies.get('typie-th-dv'),
    },
  });

  return resolve(event, {
    transformPageChunk: ({ html }) =>
      html
        .replace('%app.theme%', () => attrs.theme)
        .replace('%app.variant.light%', () => attrs.variantLight)
        .replace('%app.variant.dark%', () => attrs.variantDark),
  });
};

const header: Handle = async ({ event, resolve }) => {
  return resolve(event, {
    filterSerializedResponseHeaders: (name) => {
      const n = name.toLowerCase();

      return n === 'content-type';
    },
  });
};

const errorHandler: HandleServerError = ({ error, status, message }) => {
  const properties = isAggregatedError(error) ? { status, message, errors: error.errors } : { status, message, error };

  if (status >= 400 && status < 500) {
    log.warn('Server error {*}', properties);
  } else {
    log.error('Server error {*}', properties);
  }
};

export const handle = sequence(Sentry.sentryHandle(), logging, theme, header);
export const handleError = Sentry.handleErrorWithSentry(errorHandler);

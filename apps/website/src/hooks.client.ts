import '$lib/polyfills';
import '@typie/lib/dayjs';

import { isAggregatedError } from '@mearie/svelte';
import * as Sentry from '@sentry/sveltekit';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/public';
import { PUBLIC_IMAGE_TAG } from '$env/static/public';
import { setupMixpanel } from '$lib/analytics';
import { setupOpenTelemetry } from '$lib/otel';
import type { HandleClientError } from '@sveltejs/kit';

setupMixpanel();
setupOpenTelemetry();

Sentry.init({
  enabled: !dev,
  dsn: env.PUBLIC_SENTRY_DSN,
  environment: env.PUBLIC_ENVIRONMENT,
  release: PUBLIC_IMAGE_TAG,
  sendDefaultPii: true,
  enableLogs: true,
  tracesSampleRate: 0.1,
});

const errorHandler: HandleClientError = async ({ error }) => {
  if (isAggregatedError(error)) {
    console.error('AggregatedError:', error.errors);
  }
};

export const handleError = Sentry.handleErrorWithSentry(errorHandler);

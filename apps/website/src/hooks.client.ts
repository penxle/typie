import '$lib/polyfills';
import '@typie/lib/dayjs';

import { isAggregatedError } from '@mearie/svelte';
import * as Sentry from '@sentry/sveltekit';
import { env } from '$env/dynamic/public';
import { setupMixpanel } from '$lib/analytics';
import type { HandleClientError } from '@sveltejs/kit';

setupMixpanel();

Sentry.init({
  dsn: env.PUBLIC_SENTRY_DSN,
  sendDefaultPii: true,
});

const errorHandler: HandleClientError = async ({ error }) => {
  if (isAggregatedError(error)) {
    console.error('AggregatedError:', error.errors);
  }
};

export const handleError = Sentry.handleErrorWithSentry(errorHandler);

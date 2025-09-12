import '$lib/polyfills';
import '@typie/lib/dayjs';

import * as Sentry from '@sentry/sveltekit';
import { env } from '$env/dynamic/public';
import { setupMixpanel } from '$lib/analytics';

setupMixpanel();

Sentry.init({
  dsn: env.PUBLIC_SENTRY_DSN,
  sendDefaultPii: true,
});

export const handleError = Sentry.handleErrorWithSentry();

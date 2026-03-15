import * as Sentry from '@sentry/node';
import { dev, env, stack } from '#/env.ts';

Sentry.init({
  enabled: !dev,
  dsn: env.SENTRY_DSN,
  environment: stack,
  release: env.IMAGE_TAG,
  sendDefaultPii: true,
  enableLogs: true,
  tracesSampleRate: 0.1,
});

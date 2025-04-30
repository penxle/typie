import * as Sentry from '@sentry/bun';
import { dev, env, stack } from '@/env';

Sentry.init({
  enabled: !dev,
  dsn: env.SENTRY_DSN,
  environment: stack,
});

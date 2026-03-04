import { AsyncLocalStorage } from 'node:async_hooks';
import { configureSync, getAnsiColorFormatter, getConsoleSink, getJsonLinesFormatter, getLogger } from '@logtape/logtape';
import { getSentrySink } from '@logtape/sentry';

const production = process.env.NODE_ENV === 'production';

configureSync({
  reset: true,
  sinks: {
    console: getConsoleSink({
      formatter: production ? getJsonLinesFormatter({ message: 'template' }) : getAnsiColorFormatter({ level: 'FULL', timestamp: 'time' }),
    }),
    sentry: getSentrySink({ enableBreadcrumbs: true }),
  },
  loggers: [
    { category: 'app', lowestLevel: production ? 'info' : 'debug', sinks: ['console', 'sentry'] },
    { category: ['logtape', 'meta'], lowestLevel: 'warning', sinks: ['console'] },
  ],
  contextLocalStorage: new AsyncLocalStorage(),
});

export const logger = getLogger('app');

export { withContext } from '@logtape/logtape';

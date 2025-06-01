import { configureSync, getAnsiColorFormatter, getConsoleSink, getLogger } from '@logtape/logtape';

configureSync({
  reset: true,
  sinks: {
    console: getConsoleSink({
      formatter: getAnsiColorFormatter({
        level: 'FULL',
        timestamp: 'time',
      }),
    }),
  },
  loggers: [
    { category: 'app', lowestLevel: process.env.NODE_ENV === 'production' ? 'info' : 'debug', sinks: ['console'] },
    { category: ['logtape', 'meta'], lowestLevel: 'warning', sinks: ['console'] },
  ],
});

export const logger = getLogger('app');

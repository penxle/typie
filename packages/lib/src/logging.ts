import { configureSync, getAnsiColorFormatter, getConsoleSink, getJsonLinesFormatter, getLogger } from '@logtape/logtape';

const production = process.env.NODE_ENV === 'production';

configureSync({
  reset: true,
  sinks: {
    console: getConsoleSink({
      formatter: production ? getJsonLinesFormatter({ message: 'template' }) : getAnsiColorFormatter({ level: 'FULL', timestamp: 'time' }),
    }),
  },
  loggers: [
    { category: 'app', lowestLevel: production ? 'info' : 'debug', sinks: ['console'] },
    { category: ['logtape', 'meta'], lowestLevel: 'warning', sinks: ['console'] },
  ],
});

export const logger = getLogger('app');

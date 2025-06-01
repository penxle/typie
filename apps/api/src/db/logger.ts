import { logger } from '@typie/lib';
import { dev } from '@/env';
import type { Logger } from 'drizzle-orm/logger';

const log = logger.getChild('db');

export class DrizzleLogger implements Logger {
  logQuery(query: string, params: unknown[]): void {
    if (!dev) {
      return;
    }

    const interpolatedQuery = query
      .replaceAll(/\$(\d+)/g, (_, a) => {
        const param = params[a - 1];
        return typeof param === 'string' ? `'${param}'` : String(param);
      })
      .replaceAll('"', '');

    log.debug('Executed query: {query}', {
      query: interpolatedQuery.slice(0, 1000),
    });
  }
}

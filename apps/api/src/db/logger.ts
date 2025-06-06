import { logger } from '@typie/lib';
import { dev } from '@/env';
import type { Logger } from 'drizzle-orm/logger';

const log = logger.getChild('db');

export class DrizzleLogger implements Logger {
  logQuery(query: string, params: unknown[]): void {
    if (!dev) {
      return;
    }

    log.debug('Executed query {*}', { query, params });
  }
}

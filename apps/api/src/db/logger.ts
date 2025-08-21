import { logger } from '@typie/lib';
import { dev } from '@/env';
import type { Logger } from 'drizzle-orm/logger';

const log = logger.getChild('db');

export class DrizzleLogger implements Logger {
  logQuery(query: string, params: unknown[]): void {
    return;
    if (!dev) {
      return;
    }

    log.debug('Executed query {*}', {
      query,
      params: params.map((param) =>
        param instanceof Uint8Array || param instanceof Buffer ? `[${param.constructor.name}(${param.length} bytes)]` : param,
      ),
    });
  }
}

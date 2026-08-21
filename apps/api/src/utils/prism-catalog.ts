import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { prism, PrismApiError } from '#/external/prism.ts';
import { createTtlCache } from './prism-catalog-core.ts';
import type { PrismCommand } from '#/external/prism-core.ts';

const log = logger.getChild('prism');

const catalog = createTtlCache({ load: () => prism.getCatalog(), ttlMs: 60_000, failureTtlMs: 30_000 });

export const prismCommands = async (): Promise<PrismCommand[] | null> => {
  try {
    const { commands } = await catalog();
    return commands;
  } catch (err) {
    log.warn('prism catalog unavailable: {*}', { error: err });
    if (!(err instanceof PrismApiError) || err.code === 'malformed-response') Sentry.captureException(err);
    return null;
  }
};

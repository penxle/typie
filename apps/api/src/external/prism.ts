import { logger } from '@typie/lib';
import { env } from '#/env.ts';
import { createPrismClient } from './prism-core.ts';
import { createPrismHttp } from './prism-http.ts';

export { activeRun, newAgentId, PrismApiError, sessionTitleFrom } from './prism-core.ts';

const log = logger.getChild('prism');

export const prism = createPrismClient(
  createPrismHttp({
    baseUrl: env.PRISM_API_URL,
    token: env.PRISM_API_TOKEN,
    onRetry: ({ method, path, attempt, error }) => {
      log.warn('retrying {method} {path} (attempt {attempt}): {error}', {
        method,
        path,
        attempt,
        error: `${error.name}: ${error.message}`,
      });
    },
  }),
);

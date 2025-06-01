import { logger } from '../logging.ts';
import type { Handle } from '@sveltejs/kit';

export const logging: Handle = async ({ event, resolve }) => {
  logger.info({
    scope: 'http',
    ip: event.getClientAddress(),
    method: event.request.method,
    host: event.url.hostname,
    path: event.url.pathname,
    ua: event.request.headers.get('user-agent'),
  });

  return await resolve(event);
};

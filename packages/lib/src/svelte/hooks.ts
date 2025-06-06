import { logger } from '../logging.ts';
import type { Handle } from '@sveltejs/kit';

const log = logger.getChild('http');

export const logging: Handle = async ({ event, resolve }) => {
  log.info('Handled request {*}', {
    method: event.request.method,
    host: event.url.hostname,
    path: event.url.pathname,
    ip: event.getClientAddress(),
    ua: event.request.headers.get('user-agent'),
  });

  return await resolve(event);
};

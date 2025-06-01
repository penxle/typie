import { logger } from '../logging.ts';
import type { Handle } from '@sveltejs/kit';

const log = logger.getChild('http');

export const logging: Handle = async ({ event, resolve }) => {
  log.info('Request: {method} {host} {path} from {ip} ({ua})', {
    ip: event.getClientAddress(),
    method: event.request.method,
    host: event.url.hostname,
    path: event.url.pathname,
    ua: event.request.headers.get('user-agent'),
  });

  return await resolve(event);
};

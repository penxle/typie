import './common';

import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import type { HandleFetch, HandleServerError } from '@sveltejs/kit';

export const handle = sequence(logging);

export const handleFetch: HandleFetch = async ({ event, request, fetch }) => {
  request.headers.set('x-sveltekit-ip', event.getClientAddress());
  return fetch(request);
};

export const handleError: HandleServerError = ({ event, error, status, message }) => {
  logger.error({
    scope: 'http',
    ip: event.getClientAddress(),
    method: event.request.method,
    host: event.url.hostname,
    path: event.url.pathname,
    ua: event.request.headers.get('user-agent'),
    status,
    message,
    error,
  });
};

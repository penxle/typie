import './common';

import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import { env } from '$env/dynamic/public';
import type { HandleFetch, HandleServerError } from '@sveltejs/kit';

export const handle = sequence(logging);

export const handleFetch: HandleFetch = async ({ event, request, fetch }) => {
  const url = new URL(request.url);

  if (url.origin === env.PUBLIC_API_URL) {
    request.headers.set('x-sveltekit-ip', event.getClientAddress());
    return await fetch(request);
  }

  return await fetch(request);
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

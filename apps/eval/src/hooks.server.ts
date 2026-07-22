import { error } from '@sveltejs/kit';
import { resolveAuth } from '$lib/server/auth.ts';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  if (!event.platform) {
    error(500, 'platform unavailable');
  }

  const auth = resolveAuth({
    pathname: event.url.pathname,
    authorizationHeader: event.request.headers.get('authorization'),
    accessEmailHeader: event.request.headers.get('cf-access-authenticated-user-email'),
    ingestToken: event.platform.env.INGEST_TOKEN,
    devEmail: event.platform.env.DEV_EMAIL,
    adminEmails: event.platform.env.ADMIN_EMAILS,
  });

  if (auth.kind === 'denied') {
    error(auth.status, 'unauthorized');
  }

  event.locals.email = auth.kind === 'evaluator' ? auth.email : 'runner';

  return resolve(event);
};

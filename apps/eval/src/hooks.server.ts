import { error } from '@sveltejs/kit';
import { resolveAuth } from '$lib/server/auth.ts';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  if (!event.platform) {
    error(500, 'platform unavailable');
  }

  const auth = resolveAuth({
    pathname: event.url.pathname,
    accessEmailHeader: event.request.headers.get('cf-access-authenticated-user-email'),
    devEmail: event.platform.env.DEV_EMAIL,
    adminEmails: event.platform.env.ADMIN_EMAILS,
  });

  if (auth.kind === 'denied') {
    error(auth.status, 'unauthorized');
  }

  event.locals.email = auth.email;

  return resolve(event);
};

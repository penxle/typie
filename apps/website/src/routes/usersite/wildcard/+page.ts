import { error, redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import { legacyHomeRedirect } from '$lib/usersite/legacy-redirect';

export const load = async (event) => {
  const target = legacyHomeRedirect({
    protocol: event.url.protocol,
    usersiteHost: env.PUBLIC_USERSITE_HOST,
    host: event.url.host,
    search: event.url.search,
  });

  if (!target) error(404);

  redirect(302, target);
};

import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import { legacyEntityRedirect } from '$lib/usersite/legacy-redirect';

export const load = async (event) => {
  redirect(
    302,
    legacyEntityRedirect({
      protocol: event.url.protocol,
      usersiteHost: env.PUBLIC_USERSITE_HOST,
      slug: event.params.slug,
      search: event.url.search,
    }),
  );
};

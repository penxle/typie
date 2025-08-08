import { redirect } from '@sveltejs/kit';
import { serializeOAuthState } from '@typie/ui/utils';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async () => {
  const url = qs.stringifyUrl({
    url: `${env.PUBLIC_AUTH_URL}/authorize`,
    query: {
      client_id: env.PUBLIC_OIDC_CLIENT_ID,
      response_type: 'code',
      redirect_uri: `${env.PUBLIC_WEBSITE_URL}/authorize`,
      state: serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
    },
  });

  redirect(302, url);
};

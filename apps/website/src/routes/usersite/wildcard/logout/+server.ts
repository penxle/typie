import { redirect } from '@sveltejs/kit';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, cookies }) => {
  const redirectUri = url.searchParams.get('redirect_uri') ?? env.PUBLIC_WEBSITE_URL;

  cookies.delete('typie-at', { path: '/' });

  redirect(
    302,
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/logout`,
      query: { redirect_uri: redirectUri },
    }),
  );
};

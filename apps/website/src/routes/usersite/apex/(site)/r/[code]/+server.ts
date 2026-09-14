import { redirect } from '@sveltejs/kit';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params }) => {
  redirect(
    302,
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/login`,
      query: {
        r: params.code,
      },
    }),
  );
};

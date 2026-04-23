import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import { resolveAppStoreUrl } from '$lib/server/app-redirect';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ request }) => {
  const userAgent = request.headers.get('user-agent');
  redirect(302, resolveAppStoreUrl(userAgent, env.PUBLIC_AUTH_URL));
};

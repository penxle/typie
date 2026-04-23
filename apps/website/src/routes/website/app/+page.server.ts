import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import { resolveAppStoreUrl } from '$lib/server/app-redirect';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ request }) => {
  const userAgent = request.headers.get('user-agent');
  redirect(302, resolveAppStoreUrl(userAgent, env.PUBLIC_WEBSITE_URL));
};

import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async () => {
  redirect(301, env.PUBLIC_WEBSITE_URL);
};

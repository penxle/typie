import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';

export const GET = async () => {
  redirect(301, env.PUBLIC_WEBSITE_URL);
};

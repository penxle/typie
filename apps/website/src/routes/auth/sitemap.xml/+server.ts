import { sitemap } from '@typie/lib/svelte';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  return sitemap(event, ['/', '/login', '/signup']);
};

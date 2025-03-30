import { sitemap } from '@typie/lib/svelte';

export const GET = async (event) => {
  return sitemap(event, ['/']);
};

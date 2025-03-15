import { sitemap } from '@glitter/lib/svelte';

export const GET = async (event) => {
  return sitemap(event, ['/']);
};

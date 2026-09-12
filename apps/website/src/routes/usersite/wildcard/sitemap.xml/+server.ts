import { sitemap } from '@typie/lib/svelte';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const GET = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteWildcardSitemap_Query($origin: String!) {
        spaceView(origin: $origin) {
          id
          sitemap
        }
      }
    `),
    {
      origin: event.url.origin,
    },
  );

  return sitemap(event, [...query.data.spaceView.sitemap]);
};

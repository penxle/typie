import { sitemap } from '@typie/lib/svelte';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import { spaceHomePath } from '../paths';

export const GET = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteSpaceSitemap_Query($slug: String!) {
        siteView(slug: $slug) {
          id
          sitemap
        }
      }
    `),
    {
      slug: event.params.slug,
    },
  );

  const base = spaceHomePath(event.params.slug);
  return sitemap(
    event,
    query.data.siteView.sitemap.filter((path) => path !== '/').map((path) => `${base}${path}`),
  );
};

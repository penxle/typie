import { sitemapIndex } from '@typie/lib/svelte';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import { spaceHomePath } from '../../@[slug]/paths';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteApexSitemapIndex_Query {
        sitemapSpaceSlugs
      }
    `),
    {},
  );

  return sitemapIndex(event, ['/sitemap-root.xml', ...query.data.sitemapSpaceSlugs.map((slug) => `${spaceHomePath(slug)}/sitemap.xml`)]);
};

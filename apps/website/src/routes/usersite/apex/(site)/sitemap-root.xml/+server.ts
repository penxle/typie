import { sitemap } from '@typie/lib/svelte';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import { spaceHomePath } from '../../@[slug]/paths';
import { discoveryTagPath } from '../paths';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteApexSitemapRoot_Query {
        sitemapSpaceSlugs
        discovery {
          tags {
            name
          }
        }
      }
    `),
    {},
  );

  return sitemap(event, [
    '/',
    ...query.data.sitemapSpaceSlugs.map((slug) => spaceHomePath(slug)),
    ...query.data.discovery.tags.map((tag) => discoveryTagPath(tag.name)),
  ]);
};

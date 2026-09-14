import { sitemap } from '@typie/lib/svelte';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import { tagPath } from '../../wildcard/paths';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteApexSitemap_Query {
        discovery {
          tags {
            name
          }
        }
      }
    `),
    {},
  );

  return sitemap(event, ['/', ...query.data.discovery.tags.map((tag) => tagPath(tag.name))]);
};

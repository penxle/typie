import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query InitialPage_Query {
        me @required {
          id
          preferences

          sites {
            id

            firstEntity(type: POST) {
              id
              slug
            }
          }

          recentlyViewedEntities {
            id
            slug
          }
        }
      }
    `),
  );

  if (query.data.me.preferences.initialPage === 'home') {
    redirect(302, '/home');
  }

  const recentEntity = query.data.me.recentlyViewedEntities[0];
  if (recentEntity) {
    redirect(302, `/${recentEntity.slug}`);
  }

  const firstEntity = query.data.me.sites[0].firstEntity;
  if (firstEntity) {
    redirect(302, `/${firstEntity.slug}`);
  }

  redirect(302, '/home');
};

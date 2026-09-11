import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query IndexPage_Query {
        me {
          id
        }

        landingStats {
          ...IndexPage_Hero_landingStats
        }
      }
    `),
  );

  if (query.data.me) {
    redirect(302, '/initial');
  }

  return { query };
};

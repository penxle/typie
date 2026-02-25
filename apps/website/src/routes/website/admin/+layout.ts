import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const ssr = false;

export const load = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query AdminLayout_Query {
        me @required {
          id
          name
          email
          role

          avatar {
            id
            url
          }
        }

        ...AdminImpersonateBanner_query
      }
    `),
  );

  if (!query.data.me || query.data.me.role !== 'ADMIN') {
    redirect(302, '/initial');
  }

  return { query };
};

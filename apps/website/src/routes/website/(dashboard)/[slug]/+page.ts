import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { PageLoad } from './$types';

export const load: PageLoad = async (event) => {
  const slug = event.params.slug;
  const isHome = slug === 'home';

  const query = await loadQuery(
    event,
    graphql(`
      query DashboardSlugPage_Query($slug: String!, $isHome: Boolean!) {
        me @required {
          id
        }

        entity(slug: $slug) @skip(if: $isHome) {
          slug

          site {
            id
          }

          node {
            __typename

            ... on Document {
              id
              state {
                __typename
              }
            }
          }
        }

        ...WidgetGroup_query
      }
    `),
    { slug, isHome },
  );

  return { query };
};

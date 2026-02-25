import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query HomePage_Query {
          me @required {
            id
            name

            ...DashboardLayout_Stats_ActivityGrid_user

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
              type

              node {
                __typename

                ... on Post {
                  id
                  title
                  subtitle
                  type
                  excerpt
                }

                ... on Document {
                  id
                  title
                  documentType: type
                  excerpt
                }
              }
            }
          }
        }
      `),
    ),
  };
};

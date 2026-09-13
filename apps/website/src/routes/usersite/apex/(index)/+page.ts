import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    feedQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteApexIndexPage_Query {
          discovery {
            tags {
              name
              count
            }

            publications {
              hasMore

              publications {
                id

                space {
                  id
                  name
                  url
                }

                ...UsersiteWildcard_PublicationListItem_publicationView
              }
            }
          }
        }
      `),
      {},
    ),
  };
};

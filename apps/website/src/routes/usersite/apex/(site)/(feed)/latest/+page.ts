import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    latestQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteApexLatestPage_Query {
          discovery {
            publications {
              hasMore

              publications {
                id

                space {
                  id
                }

                ...UsersiteApex_DiscoveryCard_publicationView
              }
            }
          }
        }
      `),
      {},
    ),
  };
};

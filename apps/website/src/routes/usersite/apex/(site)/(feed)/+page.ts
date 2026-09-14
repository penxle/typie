import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const feedQuery = await loadQuery(
    event,
    graphql(`
      query UsersiteApexIndexPage_Query {
        discovery {
          feed {
            hasMore

            publications {
              id

              ...UsersiteApex_DiscoveryCard_publicationView
            }
          }
        }
      }
    `),
    {},
  );

  return { feedQuery };
};

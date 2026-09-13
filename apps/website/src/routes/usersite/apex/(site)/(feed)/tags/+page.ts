import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    tagsQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteApexTagsPage_Query {
          discovery {
            allTags {
              name
              count
            }
          }
        }
      `),
      {},
    ),
  };
};

import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query SignUpPage_Query {
          randomName
        }
      `),
    ),
  };
};

import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query OpenPage_Query {
        stats
      }
    `),
  );

  return { query };
};

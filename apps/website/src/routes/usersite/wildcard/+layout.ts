import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    layoutQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcardLayout_Query($origin: String!) {
          spaceView(origin: $origin) {
            id
            ...UsersiteHeader_spaceView
          }
        }
      `),
      {
        origin: event.url.origin,
      },
    ),
  };
};

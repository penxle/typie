import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    seriesQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcardSeriesPage_Query($origin: String!, $collectionId: ID!) {
          spaceView(origin: $origin) {
            id
            name
            dateDisplay

            collection(collectionId: $collectionId) {
              id
              name
              description
              ...UsersiteWildcard_SeriesView_collectionView
            }
          }
        }
      `),
      {
        origin: event.url.origin,
        collectionId: event.params.id,
      },
    ),
  };
};

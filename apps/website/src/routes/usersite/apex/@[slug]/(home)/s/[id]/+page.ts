import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    seriesQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceSeriesPage_Query($slug: String!, $collectionId: ID!) {
          spaceView(slug: $slug) {
            id
            name
            dateDisplay

            collection(collectionId: $collectionId) {
              id
              name
              description
              ...UsersiteSpace_SeriesView_collectionView
            }
          }
        }
      `),
      {
        slug: event.params.slug,
        collectionId: event.params.id,
      },
    ),
  };
};

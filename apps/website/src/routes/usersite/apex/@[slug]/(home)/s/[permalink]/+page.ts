import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    seriesQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceSeriesPage_Query($slug: String!, $permalink: String!) {
          spaceView(slug: $slug) {
            id
            name

            collection(permalink: $permalink) {
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
        permalink: event.params.permalink,
      },
    ),
  };
};

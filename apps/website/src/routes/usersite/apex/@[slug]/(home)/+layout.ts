import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceHomeLayout_Query($slug: String!) {
          spaceView(slug: $slug) {
            id
            name
            description
            allowIndexing
            dateDisplay
            publicationCount

            pinnedPublications {
              id
              ...UsersiteSpace_PinnedTile_publicationView
            }

            publications {
              hasMore

              publications {
                id
                ...UsersiteSpace_PublicationListItem_publicationView
              }
            }

            collections {
              id
              ...UsersiteSpace_SeriesCard_collectionView
            }

            tags {
              name
              count
            }

            ...UsersiteSpace_SpaceHeader_spaceView
            ...UsersiteSpace_SpaceRail_spaceView
          }
        }
      `),
      {
        slug: event.params.slug,
      },
    ),
  };
};

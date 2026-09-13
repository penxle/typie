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
            publicationCount

            pinnedPublications {
              id
              ...UsersiteSpace_PinnedCard_publicationView
            }

            publications {
              hasMore

              publications {
                id
                ...UsersiteApex_DiscoveryCard_publicationView
              }
            }

            collections {
              id
              permalink
              ...UsersiteSpace_SeriesCard_collectionView
            }

            tags {
              name
              count
            }

            ...UsersiteSpace_SpaceHeader_spaceView
            ...UsersiteSpace_SpaceSidebar_spaceView
          }
        }
      `),
      {
        slug: event.params.slug,
      },
    ),
  };
};

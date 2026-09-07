import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcardHomeLayout_Query($origin: String!) {
          spaceView(origin: $origin) {
            id
            name
            description
            allowIndexing
            dateDisplay
            publicationCount

            pinnedPublications {
              id
              ...UsersiteWildcard_PinnedTile_publicationView
            }

            publications {
              hasMore

              publications {
                id
                ...UsersiteWildcard_PublicationListItem_publicationView
              }
            }

            collections {
              id
              ...UsersiteWildcard_SeriesCard_collectionView
            }

            tags {
              name
              count
            }

            ...UsersiteWildcard_SpaceHeader_spaceView
            ...UsersiteWildcard_SpaceRail_spaceView
          }
        }
      `),
      {
        origin: event.url.origin,
      },
    ),
  };
};

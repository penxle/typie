import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceHomeLayout_Query($slug: String!) {
          siteView(slug: $slug) {
            id
            name
            description
            allowIndexing

            pinnedPublications {
              id
              ...UsersiteSpace_PinnedCard_publicationView
            }

            entries {
              __typename

              ... on PublicationView {
                id
                ...UsersiteApex_DiscoveryCard_publicationView
              }

              ... on SiteFolderView {
                id
                ...UsersiteSpace_FolderTile_folderView
              }
            }

            tags {
              name
              count
            }

            ...UsersiteSpace_SpaceHeader_siteView
            ...UsersiteSpace_SpaceSidebar_siteView
          }
        }
      `),
      {
        slug: event.params.slug,
      },
    ),
  };
};

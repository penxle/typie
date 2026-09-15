import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    folderQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceFolderPage_Query($slug: String!, $number: String!) {
          siteView(slug: $slug) {
            id
            name

            folder(number: $number) {
              id
              number
              name
              description
              folderCount
              publicationCount

              thumbnail {
                id
                ...Img_image
              }

              ancestors {
                id
                number
                name
              }

              children {
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
            }
          }
        }
      `),
      {
        slug: event.params.slug,
        number: event.params.number,
      },
    ),
  };
};

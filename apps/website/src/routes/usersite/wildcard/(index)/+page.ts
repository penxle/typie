import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcardIndexPage_Query($origin: String!) {
          siteView(origin: $origin) {
            id
            name
            dateDisplay

            logo {
              id
              ...Img_image
            }

            entities {
              id
              slug

              node {
                __typename

                ... on FolderView {
                  id
                  name
                  folderCount
                  documentCount
                  thumbnail {
                    id
                    ...Img_image
                  }
                }

                ... on DocumentView {
                  id
                  title
                  subtitle
                  excerpt
                  createdAt
                  updatedAt
                  thumbnail {
                    id
                    ...Img_image
                  }
                }
              }
            }
          }
        }
      `),
      {
        origin: event.url.origin,
      },
    ),
  };
};

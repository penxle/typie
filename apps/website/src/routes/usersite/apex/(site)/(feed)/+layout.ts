import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    feedLayoutQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteApexFeedLayout_Query {
          discovery {
            tags {
              name
              count
            }

            recentSpaces {
              space {
                id
                name
                url

                logo {
                  id
                  ...Img_image
                }
              }

              publication {
                id
                title
                publishedAt
              }
            }

            recentCollections {
              collection {
                id
                permalink
                name
                publicationCount

                cover {
                  id
                  ...Img_image
                }
              }

              space {
                id
                name
                url
              }
            }
          }
        }
      `),
      {},
    ),
  };
};

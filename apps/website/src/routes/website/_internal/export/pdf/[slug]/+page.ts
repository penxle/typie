import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

declare global {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Window {
    notifyFontsReady?: () => void;
  }
}

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query ExportPdfSlugPage_query($slug: String!) {
          entity(slug: $slug) {
            id

            user {
              id
              name
            }

            site {
              id

              fonts {
                id
                weight
                url

                family {
                  id
                }
              }
            }

            node {
              __typename

              ... on Post {
                id
                title
                subtitle
                body
                createdAt
              }
            }
          }
        }
      `),
      {
        slug: event.params.slug,
      },
    ),
  };
};

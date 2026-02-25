import { error } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const { url } = event;
  const slug = url.searchParams.get('slug');
  const siteId = url.searchParams.get('siteId');
  if (!slug || !siteId) {
    error(404);
  }
  return {
    slug,
    query: await loadQuery(
      event,
      graphql(`
        query WebViewEditorPage_Query($slug: String!, $siteId: ID!) {
          ...WebViewEditor_Limit_query

          me @required {
            id
            name

            subscription {
              id

              plan {
                id

                rule {
                  maxTotalCharacterCount
                  maxTotalBlobSize
                }
              }
            }
          }

          post(slug: $slug) {
            id
            update

            entity {
              id

              notes {
                id
                content
                color
              }

              site {
                id

                fonts {
                  id
                  url
                  weight

                  family {
                    id
                  }
                }
              }
            }
          }

          site(siteId: $siteId) {
            id

            usage {
              totalCharacterCount
              totalBlobSize
            }
          }
        }
      `),
      {
        slug,
        siteId,
      },
    ),
  };
};

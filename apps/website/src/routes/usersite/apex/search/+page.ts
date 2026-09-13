import { shouldDegradeSearchError } from '$lib/discovery/search-load';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const q = (event.url.searchParams.get('q') ?? '').trim();
  if (!q) {
    return { q, searchQuery: null, failed: false };
  }

  try {
    const searchQuery = await loadQuery(
      event,
      graphql(`
        query UsersiteApexSearchPage_Query($query: String!) {
          discovery {
            search(query: $query) {
              publications {
                title
                excerpt

                publication {
                  id

                  space {
                    id
                    name
                    url
                  }

                  ...UsersiteWildcard_PublicationListItem_publicationView
                }
              }

              spaces {
                id
                name
                description
                url

                logo {
                  id
                  ...Img_image
                }
              }

              tags {
                name
                count
              }
            }
          }
        }
      `),
      { query: q },
    );
    return { q, searchQuery, failed: false };
  } catch (err) {
    if (shouldDegradeSearchError(err)) {
      return { q, searchQuery: null, failed: true };
    }
    throw err;
  }
};

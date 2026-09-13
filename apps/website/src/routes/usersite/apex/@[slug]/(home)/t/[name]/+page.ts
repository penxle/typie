import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    tagQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceTagPage_Query($slug: String!, $name: String!) {
          spaceView(slug: $slug) {
            id
            name
            dateDisplay

            tags {
              name
              count
            }

            tag(name: $name) {
              name
              count

              publications {
                hasMore

                publications {
                  id
                  ...UsersiteSpace_PublicationListItem_publicationView
                }
              }
            }
          }
        }
      `),
      {
        slug: event.params.slug,
        name: event.params.name,
      },
    ),
  };
};

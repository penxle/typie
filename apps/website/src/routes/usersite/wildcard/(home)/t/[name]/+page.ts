import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    tagQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcardTagPage_Query($origin: String!, $name: String!) {
          spaceView(origin: $origin) {
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
                  ...UsersiteWildcard_PublicationListItem_publicationView
                }
              }
            }
          }
        }
      `),
      {
        origin: event.url.origin,
        name: event.params.name,
      },
    ),
  };
};

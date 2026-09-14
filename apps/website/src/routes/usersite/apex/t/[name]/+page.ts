import { error } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const tagQuery = await loadQuery(
    event,
    graphql(`
      query UsersiteApexTagPage_Query($name: String!) {
        discovery {
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

                space {
                  id
                  name
                  url
                }

                ...UsersiteWildcard_PublicationListItem_publicationView
              }
            }
          }
        }
      }
    `),
    { name: event.params.name },
  );

  if (!tagQuery.data.discovery.tag) {
    error(404, { message: '태그를 찾을 수 없어요.', code: 'not_found' });
  }

  return { tagQuery };
};

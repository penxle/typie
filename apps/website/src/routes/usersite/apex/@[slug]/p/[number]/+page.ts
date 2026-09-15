import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteSpacePublicationPage_Query($slug: String!, $number: String!) {
          me {
            id

            ...UsersiteSpacePublicationPage_PublicationViewV2_user
          }

          siteView(slug: $slug) {
            id
            allowIndexing

            publication(number: $number) {
              id
              ...UsersiteSpacePublicationPage_PublicationViewV2_publicationView
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

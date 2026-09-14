import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteSpacePublicationPage_Query($slug: String!, $publicationId: ID!) {
          me {
            id

            ...UsersiteSpacePublicationPage_PublicationViewV2_user
          }

          spaceView(slug: $slug) {
            id
            allowIndexing

            publication(publicationId: $publicationId) {
              id
              ...UsersiteSpacePublicationPage_PublicationViewV2_publicationView
            }
          }
        }
      `),
      {
        slug: event.params.slug,
        publicationId: event.params.id,
      },
    ),
  };
};

import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcardPublicationPage_Query($origin: String!, $publicationId: ID!) {
          me {
            id

            ...UsersiteWildcardPublicationPage_PublicationViewV2_user
          }

          spaceView(origin: $origin) {
            id
            allowIndexing

            publication(publicationId: $publicationId) {
              id
              ...UsersiteWildcardPublicationPage_PublicationViewV2_publicationView
            }
          }
        }
      `),
      {
        origin: event.url.origin,
        publicationId: event.params.id,
      },
    ),
  };
};

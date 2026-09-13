import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteSpacePublicationPage_Query($slug: String!, $permalink: String!) {
          me {
            id

            ...UsersiteSpacePublicationPage_PublicationViewV2_user
          }

          spaceView(slug: $slug) {
            id
            allowIndexing

            publication(permalink: $permalink) {
              id
              ...UsersiteSpacePublicationPage_PublicationViewV2_publicationView
            }
          }
        }
      `),
      {
        slug: event.params.slug,
        permalink: event.params.permalink,
      },
    ),
  };
};

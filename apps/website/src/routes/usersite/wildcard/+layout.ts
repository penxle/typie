import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query UsersiteWildcard_Layout_Query {
          me {
            id
            name
            email

            avatar {
              id
              url

              ...Img_image
            }
          }

          ...AdminImpersonateBanner_query
        }
      `),
      {
        origin: event.url.origin,
      },
    ),
  };
};

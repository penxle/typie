import { checkBootstrapAssertion } from '$lib/bootstrap';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const [, query] = await Promise.all([
    checkBootstrapAssertion(event.fetch),
    loadQuery(
      event,
      graphql(`
        query Usersite_Layout_Query {
          me {
            id
            name
            email

            avatar {
              id
              url

              ...Img_image
            }

            ...UsersiteHeader_user
          }

          ...AdminImpersonateBanner_query
        }
      `),
      {},
    ),
  ]);

  return { query };
};

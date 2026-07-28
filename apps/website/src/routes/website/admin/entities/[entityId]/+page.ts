import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminEntity_Query($entityId: String!) {
          adminEntity(entityId: $entityId) {
            id
            slug
            permalink
            url
            type
            state
            visibility
            availability
            deletedAt
            createdAt

            user {
              id
              name
              email
            }

            site {
              id
              name
            }

            ancestors {
              id

              node {
                __typename

                ... on Document {
                  title
                }

                ... on Folder {
                  name
                }
              }
            }

            node {
              __typename

              ... on Document {
                id
                title
                subtitle
                contentRating
                allowReaction
                protectContent
                password
                characterCount
                createdAt
                updatedAt

                heads {
                  id
                  updatedAt
                  characterCount

                  contributors {
                    id
                    name
                  }
                }
              }

              ... on Folder {
                id
                name
              }
            }
          }
        }
      `),
      {
        entityId: event.params.entityId,
      },
    ),
  };
};

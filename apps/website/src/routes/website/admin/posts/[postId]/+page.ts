import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminPost_Query($postId: String!) {
          adminPost(postId: $postId) {
            id
            title
            subtitle
            type
            contentRating
            allowReaction
            protectContent
            createdAt
            updatedAt
            excerpt
            password
            coverImage {
              id
              url
            }
            entity {
              id
              slug
              url
              visibility
              state
              ancestors {
                id
                node {
                  __typename
                  ... on Folder {
                    name
                  }
                  ... on Post {
                    title
                  }
                  ... on Document {
                    title
                  }
                }
              }
              user {
                id
                name
                email
                avatar {
                  id
                  url
                }
                postCount
                subscription {
                  id
                  state
                }
              }
            }
            reactionCount
            characterCount
          }
        }
      `),
      {
        postId: event.params.postId,
      },
    ),
  };
};

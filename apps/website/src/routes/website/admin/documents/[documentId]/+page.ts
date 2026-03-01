import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminDocument_Query($documentId: String!) {
          adminDocument(documentId: $documentId) {
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
            thumbnail {
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
              }
            }
            reactionCount
            characterCount
          }
        }
      `),
      {
        documentId: event.params.documentId,
      },
    ),
  };
};

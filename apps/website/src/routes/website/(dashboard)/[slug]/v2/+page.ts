import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { PageLoad } from './$types';

export const load: PageLoad = async (event) => {
  const slug = event.params.slug;

  const query = await loadQuery(
    event,
    graphql(`
      query DocumentV2Page_Query($slug: String!) {
        entity(slug: $slug) {
          id
          node {
            __typename
            ... on Document {
              id
              ...DocumentEditorV2_document
            }
          }
        }
      }
    `),
    { slug },
  );

  return { query };
};

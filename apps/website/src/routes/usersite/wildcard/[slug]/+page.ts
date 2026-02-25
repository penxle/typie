import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteWildcardSlugPage_Query($origin: String!, $slug: String!) {
        me {
          id

          ...UsersiteWildcardSlugPage_PostView_user
          ...UsersiteWildcardSlugPage_DocumentView_user
        }

        entityView(origin: $origin, slug: $slug) {
          id

          node {
            __typename

            ... on PostView {
              id

              document {
                id

                entity {
                  id
                  slug
                }
              }
            }
          }

          ...UsersiteWildcardSlugPage_DocumentView_entityView
          ...UsersiteWildcardSlugPage_FolderView_entityView
          ...UsersiteWildcardSlugPage_PostView_entityView
        }
      }
    `),
    {
      origin: event.url.origin,
      slug: event.params.slug,
    },
  );

  if (query.data.entityView.node.__typename === 'PostView' && query.data.entityView.node.document) {
    redirect(302, `/${query.data.entityView.node.document.entity.slug}`);
  }

  return { query };
};

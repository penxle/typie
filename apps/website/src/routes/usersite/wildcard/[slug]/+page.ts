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

          ...UsersiteWildcardSlugPage_DocumentView_user
        }

        entityView(origin: $origin, slug: $slug) {
          id
          slug

          node {
            __typename
          }

          ...UsersiteWildcardSlugPage_DocumentView_entityView
          ...UsersiteWildcardSlugPage_FolderView_entityView
        }
      }
    `),
    {
      origin: event.url.origin,
      slug: event.params.slug,
    },
  );

  if (query.data.entityView.slug !== event.params.slug) {
    redirect(302, `/${query.data.entityView.slug}`);
  }

  return { query };
};

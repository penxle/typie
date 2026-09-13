import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { resolveLinkShareRedirect } from '$lib/usersite/link-share-redirect';
import { graphql } from '$mearie';

export const load = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteApexSlugPage_Query($origin: String!, $slug: String!) {
        me {
          id

          ...UsersiteApexSlugPage_DocumentViewV2_user
        }

        entityView(origin: $origin, slug: $slug) {
          id
          slug

          node {
            __typename

            ... on DocumentView {
              id

              publication {
                id
                url
              }
            }
          }

          ...UsersiteApexSlugPage_DocumentViewV2_entityView
          ...UsersiteApexSlugPage_FolderView_entityView
        }
      }
    `),
    {
      origin: event.url.origin,
      slug: event.params.slug,
    },
  );

  const node = query.data.entityView.node;
  const target = resolveLinkShareRedirect({
    requestedSlug: event.params.slug,
    entitySlug: query.data.entityView.slug,
    publicationUrl: node.__typename === 'DocumentView' ? (node.publication?.url ?? null) : null,
  });

  if (target) {
    redirect(302, target);
  }

  return { query };
};

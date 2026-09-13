import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import { lowercaseSpaceRedirectPath } from './paths';

export const load = async (event) => {
  const slug = event.params.slug;
  if (slug !== slug.toLowerCase()) {
    const target = lowercaseSpaceRedirectPath(event.url.pathname, slug);
    if (target) {
      redirect(301, `${target}${event.url.search}`);
    }
  }

  return {
    layoutQuery: await loadQuery(
      event,
      graphql(`
        query UsersiteSpaceLayout_Query($slug: String!) {
          spaceView(slug: $slug) {
            id
            url
            ...UsersiteHeader_spaceView
          }
        }
      `),
      {
        slug,
      },
    ),
  };
};

import { redirect } from '@sveltejs/kit';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const GET = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteApexPermalinkPage_Query($permalink: String!) {
        permalink(permalink: $permalink) {
          entitySlug
        }
      }
    `),
    {
      permalink: event.params.permalink,
    },
  );

  redirect(302, `/s/${query.data.permalink.entitySlug}`);
};

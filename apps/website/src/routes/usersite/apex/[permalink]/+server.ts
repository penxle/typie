import { redirect } from '@sveltejs/kit';
import { serializeOAuthState } from '@typie/ui/utils';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const GET = async (event) => {
  const query = await loadQuery(
    event,
    graphql(`
      query UsersiteApexPermalinkPage_Query($permalink: String!) {
        permalink(permalink: $permalink) {
          siteUrl
          entitySlug
        }
      }
    `),
    {
      permalink: event.params.permalink,
    },
  );

  const authorizeUrl = qs.stringifyUrl({
    url: `${env.PUBLIC_AUTH_URL}/authorize`,
    query: {
      client_id: env.PUBLIC_OIDC_CLIENT_ID,
      response_type: 'code',
      redirect_uri: `${query.data.permalink.siteUrl}/authorize`,
      state: serializeOAuthState({ redirect_uri: `${query.data.permalink.siteUrl}/${query.data.permalink.entitySlug}` }),
      prompt: 'none',
    },
  });

  redirect(302, authorizeUrl);
};

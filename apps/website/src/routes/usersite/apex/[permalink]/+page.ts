import { redirect } from '@sveltejs/kit';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import { serializeOAuthState } from '$lib/utils/auth';
import type { UsersiteApexPermalinkPage_Query_AfterLoad, UsersiteApexPermalinkPage_Query_Variables } from './$graphql';

export const _UsersiteApexPermalinkPage_Query_Variables: UsersiteApexPermalinkPage_Query_Variables = ({ params }) => ({
  permalink: params.permalink,
});

export const _UsersiteApexPermalinkPage_Query_AfterLoad: UsersiteApexPermalinkPage_Query_AfterLoad = ({ query }) => {
  const authorizeUrl = qs.stringifyUrl({
    url: `${env.PUBLIC_AUTH_URL}/authorize`,
    query: {
      client_id: env.PUBLIC_OIDC_CLIENT_ID,
      response_type: 'code',
      redirect_uri: `${query.permalink.siteUrl}/authorize`,
      state: serializeOAuthState({ redirect_uri: `${query.permalink.siteUrl}/${query.permalink.entitySlug}` }),
    },
  });

  redirect(302, authorizeUrl);
};

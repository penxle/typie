import { redirect } from '@sveltejs/kit';
import { serializeOAuthState } from '@typie/ui/utils';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async (event) => {
  if (!event.data.desktop) {
    return {};
  }

  const query = await loadQuery(
    event,
    graphql(`
      query LandingLayout_Query {
        me {
          id
        }
      }
    `),
  );

  if (query.data.me) {
    redirect(302, '/initial');
  }

  redirect(
    302,
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/authorize`,
      query: {
        client_id: env.PUBLIC_OIDC_CLIENT_ID,
        response_type: 'code',
        redirect_uri: `${env.PUBLIC_WEBSITE_URL}/authorize`,
        state: serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
      },
    }),
  );
};

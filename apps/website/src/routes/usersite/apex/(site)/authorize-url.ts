import { serializeOAuthState } from '@typie/ui/utils';
import qs from 'query-string';
import { env } from '$env/dynamic/public';

export const apexAuthorizeUrl = (url: URL) =>
  qs.stringifyUrl({
    url: `${env.PUBLIC_AUTH_URL}/authorize`,
    query: {
      client_id: env.PUBLIC_OIDC_CLIENT_ID,
      response_type: 'code',
      redirect_uri: `${url.origin}/authorize`,
      state: serializeOAuthState({ redirect_uri: url.href }),
    },
  });

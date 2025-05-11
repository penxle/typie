import ky from 'ky';
import qs from 'query-string';
import { SingleSignOnProvider } from '@/enums';
import { env } from '@/env';
import type { ExternalUser } from './types';

export const generateAuthorizationUrl = (state: string) => {
  return qs.stringifyUrl({
    url: 'https://nid.naver.com/oauth2.0/authorize',
    query: {
      response_type: 'code',
      client_id: env.NAVER_CLIENT_ID,
      redirect_uri: `${env.AUTH_URL}/sso/naver`,
      state,
    },
  });
};

export const authorizeUser = async (params: Record<string, string>): Promise<ExternalUser> => {
  let accessToken = params.access_token;

  if (!accessToken) {
    if (!params.code) {
      throw new Error('Invalid parameters');
    }

    const tokens = await ky(
      qs.stringifyUrl({
        url: 'https://nid.naver.com/oauth2.0/token',
        query: {
          grant_type: 'authorization_code',
          client_id: env.NAVER_CLIENT_ID,
          client_secret: env.NAVER_CLIENT_SECRET,
          code: params.code,
        },
      }),
    ).json<{ access_token: string }>();

    if (!tokens.access_token) {
      throw new Error('Token validation failed');
    }

    accessToken = tokens.access_token;
  }

  type R = { response: { id: string; email: string; nickname: string; profile_image: string } };
  const me = await ky('https://openapi.naver.com/v1/nid/me', {
    headers: { Authorization: `Bearer ${accessToken}` },
  }).json<R>();

  return {
    provider: SingleSignOnProvider.NAVER,
    principal: me.response.id,
    email: me.response.email.toLowerCase(),
    name: me.response.nickname,
    avatarUrl: me.response.profile_image,
  };
};

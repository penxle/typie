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

export const authorizeUser = async (code: string): Promise<ExternalUser> => {
  const tokens = await ky(
    qs.stringifyUrl({
      url: 'https://nid.naver.com/oauth2.0/token',
      query: {
        grant_type: 'authorization_code',
        client_id: env.NAVER_CLIENT_ID,
        client_secret: env.NAVER_CLIENT_SECRET,
        code,
      },
    }),
  ).json<{ access_token: string }>();

  if (!tokens.access_token) {
    throw new Error('Token validation failed');
  }

  type R = { response: { id: string; email: string; nickname: string; profile_image: string } };
  const me = await ky('https://openapi.naver.com/v1/nid/me', {
    headers: { Authorization: `Bearer ${tokens.access_token}` },
  }).json<R>();

  return {
    provider: SingleSignOnProvider.NAVER,
    principal: me.response.id,
    email: me.response.email.toLowerCase(),
    name: me.response.nickname,
    avatarUrl: me.response.profile_image,
  };
};

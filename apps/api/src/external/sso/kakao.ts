import ky from 'ky';
import qs from 'query-string';
import { SingleSignOnProvider } from '@/enums';
import { env } from '@/env';
import type { ExternalUser } from './types';

export const generateAuthorizationUrl = (state: string) => {
  return qs.stringifyUrl({
    url: 'https://kauth.kakao.com/oauth/authorize',
    query: {
      response_type: 'code',
      client_id: env.KAKAO_CLIENT_ID,
      redirect_uri: `${env.AUTH_URL}/sso/kakao`,
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

    const tokens = await ky
      .post('https://kauth.kakao.com/oauth/token', {
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded;charset=utf-8',
        },
        body: qs.stringify({
          grant_type: 'authorization_code',
          client_id: env.KAKAO_CLIENT_ID,
          client_secret: env.KAKAO_CLIENT_SECRET,
          redirect_uri: `${env.AUTH_URL}/sso/kakao`,
          code: params.code,
        }),
      })
      .json<{ access_token: string }>();

    if (!tokens.access_token) {
      throw new Error('Token validation failed');
    }

    accessToken = tokens.access_token;
  }

  type KakaoUserResponse = {
    id: number;
    kakao_account: {
      email: string;
      is_email_valid: boolean;
      is_email_verified: boolean;
      profile: {
        nickname: string;
        profile_image_url: string;
        is_default_image: boolean;
      };
    };
  };

  const me = await ky('https://kapi.kakao.com/v2/user/me', {
    headers: {
      Authorization: `Bearer ${accessToken}`,
      'Content-Type': 'application/x-www-form-urlencoded;charset=utf-8',
    },
  }).json<KakaoUserResponse>();

  if (!me.kakao_account.is_email_valid || !me.kakao_account.is_email_verified) {
    throw new Error('Email validation failed');
  }

  return {
    provider: SingleSignOnProvider.KAKAO,
    principal: me.id.toString(),
    email: me.kakao_account.email.toLowerCase(),
    name: me.kakao_account.profile.nickname,
    avatarUrl: me.kakao_account.profile.is_default_image ? null : me.kakao_account.profile.profile_image_url,
  };
};

import { SingleSignOnProvider } from '@typie/lib/enums';
import { OAuth2Client } from 'google-auth-library';
import { env } from '#/env.ts';
import type { ExternalUser } from './types.ts';

const createOAuthClient = () => {
  return new OAuth2Client({
    clientId: env.GOOGLE_OAUTH_CLIENT_ID,
    clientSecret: env.GOOGLE_OAUTH_CLIENT_SECRET,
    redirectUri: `${env.AUTH_URL}/sso/google`,
  });
};

export const generateAuthorizationUrl = (state: string, email?: string | null) => {
  const client = createOAuthClient();
  return client.generateAuthUrl({
    scope: ['email', 'profile'],
    state,
    login_hint: email ?? undefined,
    prompt: 'select_account',
  });
};

export const authorizeUser = async (params: Record<string, string>): Promise<ExternalUser> => {
  const client = createOAuthClient();

  if (params.token) {
    const ticket = await client.verifyIdToken({
      idToken: params.token,
      audience: env.GOOGLE_OAUTH_CLIENT_ID,
    });

    const payload = ticket.getPayload();
    if (!payload?.sub || !payload.email) {
      throw new Error('Token validation failed');
    }

    return {
      provider: SingleSignOnProvider.GOOGLE,
      principal: payload.sub,
      email: payload.email.toLowerCase(),
      name: payload.name ?? null,
      avatarUrl: payload.picture ?? null,
    };
  }

  if (params.code) {
    const { tokens } = await client.getToken(params.code);
    if (!tokens.access_token) {
      throw new Error('Token validation failed');
    }

    const { aud } = await client.getTokenInfo(tokens.access_token);
    if (aud !== env.GOOGLE_OAUTH_CLIENT_ID) {
      throw new Error('Token validation failed');
    }

    client.setCredentials(tokens);

    type R = { sub: string; email: string; name: string; picture: string };
    const userinfo = await client.request<R>({
      url: 'https://www.googleapis.com/oauth2/v3/userinfo',
    });

    return {
      provider: SingleSignOnProvider.GOOGLE,
      principal: userinfo.data.sub,
      email: userinfo.data.email.toLowerCase(),
      name: userinfo.data.name,
      avatarUrl: userinfo.data.picture,
    };
  }

  throw new Error('Invalid parameters');
};

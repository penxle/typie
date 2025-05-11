import { OAuth2Client } from 'google-auth-library';
import { SingleSignOnProvider } from '@/enums';
import { env } from '@/env';
import type { ExternalUser } from './types';

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
  });
};

export const authorizeUser = async (params: Record<string, string>): Promise<ExternalUser> => {
  if (!params.code) {
    throw new Error('Invalid parameters');
  }

  const client = createOAuthClient();

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
};

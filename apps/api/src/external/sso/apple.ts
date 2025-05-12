import appleSignIn from 'apple-signin-auth';
import { SingleSignOnProvider } from '@/enums';
import { env } from '@/env';
import type { ExternalUser } from './types';

export const generateAuthorizationUrl = () => {
  throw new Error('Not implemented');
};

export const authorizeUser = async (params: Record<string, string>): Promise<ExternalUser> => {
  if (!params.code) {
    throw new Error('Invalid parameters');
  }

  const clientSecret = appleSignIn.getClientSecret({
    clientID: env.APPLE_APP_BUNDLE_ID,
    teamID: env.APPLE_TEAM_ID,
    keyIdentifier: env.APPLE_SIGN_IN_KEY_ID,
    privateKey: env.APPLE_SIGN_IN_PRIVATE_KEY,
  });

  const tokens = await appleSignIn.getAuthorizationToken(params.code, {
    clientID: env.APPLE_APP_BUNDLE_ID,
    clientSecret,
    redirectUri: `${env.AUTH_URL}/sso/apple`,
  });

  if (!tokens.id_token) {
    throw new Error('Token validation failed');
  }

  const idToken = await appleSignIn.verifyIdToken(tokens.id_token, {
    audience: env.APPLE_APP_BUNDLE_ID,
  });

  return {
    provider: SingleSignOnProvider.APPLE,
    principal: idToken.sub,
    email: idToken.email.toLowerCase(),
    name: null,
    avatarUrl: null,
  };
};

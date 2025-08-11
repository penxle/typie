import { createHash } from 'node:crypto';
import dayjs from 'dayjs';
import { and, eq, gt } from 'drizzle-orm';
import escape from 'escape-string-regexp';
import { Hono } from 'hono';
import { deleteCookie, getCookie } from 'hono/cookie';
import * as jose from 'jose';
import { nanoid } from 'nanoid';
import qs from 'query-string';
import { base64url } from 'rfc4648';
import { redis } from '@/cache';
import { db, first, UserSessions } from '@/db';
import { env } from '@/env';
import { jwk, privateKey, publicKey } from '@/utils';
import type { Env } from '@/context';

export const auth = new Hono<Env>();

auth.get('/.well-known/openid-configuration', (c) => {
  return c.json({
    issuer: env.AUTH_URL,
    authorization_endpoint: `${env.AUTH_URL}/authorize`,
    token_endpoint: `${env.AUTH_URL}/token`,
    userinfo_endpoint: `${env.AUTH_URL}/userinfo`,
    jwks_uri: `${env.AUTH_URL}/jwks`,
    response_types_supported: ['code'],
    subject_types_supported: ['public'],
    id_token_signing_alg_values_supported: [jwk.alg],
    scopes_supported: ['openid'],
    token_endpoint_auth_methods_supported: ['client_secret_post'],
    claim_types_supported: ['normal'],
    claims_supported: ['sub'],
    code_challenge_methods_supported: ['S256'],
  });
});

auth.get('/jwks', async (c) => {
  const exported = await jose.exportJWK(publicKey);
  return c.json({
    keys: [{ ...exported, kid: jwk.kid, alg: jwk.alg }],
  });
});

auth.get('/authorize', async (c) => {
  const { client_id, redirect_uri, response_type, scope, state, prompt, code_challenge, code_challenge_method } = c.req.query();

  if (!client_id || !redirect_uri || !response_type) {
    return c.json({ error: 'invalid_request', error_description: 'Required parameters are missing.' }, 400);
  }

  if (response_type !== 'code') {
    return c.json({ error: 'unsupported_response_type', error_description: 'Only code response type is supported.' }, 400);
  }

  if (!validateClient({ clientId: client_id, redirectUri: redirect_uri })) {
    return c.json({ error: 'invalid_client', error_description: 'Client validation failed.' }, 400);
  }

  if (code_challenge && code_challenge_method !== 'S256') {
    return c.json({ error: 'invalid_request', error_description: 'Only S256 code_challenge_method is supported.' }, 400);
  }

  const token = getCookie(c, 'typie-st');

  if (token) {
    const session = await db
      .select({
        id: UserSessions.id,
        userId: UserSessions.userId,
      })
      .from(UserSessions)
      .where(and(eq(UserSessions.token, token), gt(UserSessions.expiresAt, dayjs())))
      .then(first);

    if (session) {
      const code = nanoid(32);

      const authCode: AuthorizationCode = {
        sessionId: session.id,
        clientId: client_id,
        redirectUri: redirect_uri,
        scope: scope || 'openid',
        codeChallenge: code_challenge,
        codeChallengeMethod: code_challenge_method,
      };

      await redis.setex(`auth:code:${code}`, 60 * 10, JSON.stringify(authCode));

      return c.redirect(
        qs.stringifyUrl({
          url: redirect_uri,
          query: {
            code,
            state,
          },
        }),
      );
    }
  }

  if (prompt === 'none') {
    return c.redirect(
      qs.stringifyUrl({
        url: redirect_uri,
        query: {
          error: 'login_required',
          error_description: 'User authentication is required.',
          state,
        },
      }),
    );
  }

  return c.redirect(
    qs.stringifyUrl({
      url: `${env.AUTH_URL}/login`,
      query: {
        redirect_uri,
        state,
      },
    }),
  );
});

auth.post('/token', async (c) => {
  const { grant_type, code, redirect_uri, client_id, client_secret, code_verifier } = await c.req.parseBody<Record<string, string>>();

  if (!client_id) {
    return c.json({ error: 'invalid_client', error_description: 'Client ID is missing.' }, 401);
  }

  if (!validateClient({ clientId: client_id, clientSecret: client_secret, redirectUri: redirect_uri })) {
    return c.json({ error: 'invalid_client', error_description: 'Client authentication failed.' }, 401);
  }

  if (grant_type === 'authorization_code') {
    if (!code || !redirect_uri) {
      return c.json({ error: 'invalid_request', error_description: 'Code and redirect_uri are required parameters.' }, 400);
    }

    const result = await redis.getdel(`auth:code:${code}`);
    if (!result) {
      return c.json({ error: 'invalid_grant', error_description: 'Invalid or expired authorization code.' }, 400);
    }

    const authCode = JSON.parse(result) as AuthorizationCode;

    if (authCode.clientId !== client_id || authCode.redirectUri !== redirect_uri) {
      return c.json({ error: 'invalid_grant', error_description: 'Authorization code parameters do not match.' }, 400);
    }

    if (authCode.codeChallenge) {
      if (!code_verifier) {
        return c.json({ error: 'invalid_request', error_description: 'code_verifier is required.' }, 400);
      }

      const hash = createHash('sha256').update(code_verifier).digest();
      const codeChallenge = base64url.stringify(hash, { pad: false });

      if (codeChallenge !== authCode.codeChallenge) {
        return c.json({ error: 'invalid_grant', error_description: 'code_verifier is invalid.' }, 400);
      }
    }

    const session = await db
      .select({
        id: UserSessions.id,
        userId: UserSessions.userId,
      })
      .from(UserSessions)
      .where(and(eq(UserSessions.id, authCode.sessionId), gt(UserSessions.expiresAt, dayjs())))
      .then(first);

    if (!session) {
      return c.json({ error: 'invalid_grant', error_description: 'Session has expired or does not exist.' }, 400);
    }

    const expiresIn = dayjs.duration(1, 'year');
    const expiresAt = dayjs().add(expiresIn);

    const now = dayjs().unix();

    const accessToken = await new jose.SignJWT({
      iss: env.AUTH_URL,
      sub: session.userId,
      aud: client_id,
      exp: expiresAt.unix(),
      iat: now,
      sid: session.id,
      scope: authCode.scope,
    })
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      .setProtectedHeader({ alg: jwk.alg!, kid: jwk.kid })
      .sign(privateKey);

    let idToken;

    if (authCode.scope.includes('openid')) {
      idToken = await new jose.SignJWT({
        iss: env.AUTH_URL,
        sub: session.userId,
        aud: client_id,
        exp: expiresAt.unix(),
        iat: now,
      })
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        .setProtectedHeader({ alg: jwk.alg!, kid: jwk.kid })
        .sign(privateKey);
    }

    return c.json({
      access_token: accessToken,
      token_type: 'Bearer',
      expires_in: expiresIn.asSeconds(),
      scope: authCode.scope,
      id_token: idToken,
    });
  }

  return c.json({ error: 'unsupported_grant_type', error_description: 'Only authorization_code grant type is supported.' }, 400);
});

auth.get('/userinfo', async (c) => {
  const authorization = c.req.header('Authorization');
  const accessToken = authorization?.match(/^Bearer\s+(.+)$/)?.[1];

  if (!accessToken) {
    return c.json({ error: 'invalid_token', error_description: 'Missing or invalid Authorization header.' }, 401);
  }

  try {
    const { payload } = await jose.jwtVerify(accessToken, publicKey);

    if (!payload.sub) {
      return c.json({ error: 'invalid_token', error_description: 'Token missing sub claim.' }, 401);
    }

    return c.json({ sub: payload.sub });
  } catch {
    return c.json({ error: 'invalid_token', error_description: 'Invalid or expired access token.' }, 401);
  }
});

auth.get('/logout', async (c) => {
  const { redirect_uri } = c.req.query();

  if (!redirect_uri) {
    return c.json({ error: 'invalid_request', error_description: 'redirect_uri is required.' }, 400);
  }

  const token = getCookie(c, 'typie-st');
  if (!token) {
    return c.json({ error: 'invalid_request', error_description: 'No active session found.' }, 400);
  }

  const session = await db.select({ id: UserSessions.id }).from(UserSessions).where(eq(UserSessions.token, token)).then(first);

  if (!session) {
    return c.json({ error: 'invalid_request', error_description: 'Invalid session.' }, 400);
  }

  await db.delete(UserSessions).where(eq(UserSessions.id, session.id));

  deleteCookie(c, 'typie-st');

  return c.redirect(redirect_uri);
});

type AuthorizationCode = {
  sessionId: string;
  clientId: string;
  redirectUri: string;
  scope: string;
  codeChallenge?: string;
  codeChallengeMethod?: string;
};

type ValidateClientParams = {
  clientId: string;
  clientSecret?: string;
  redirectUri: string;
};

const pattern = new RegExp(`^${escape(env.USERSITE_URL).replace(String.raw`\*\.`, String.raw`(([^.]+)\.)?`)}$`);
const validateClient = ({ clientId, clientSecret, redirectUri }: ValidateClientParams) => {
  if (clientId !== env.OIDC_CLIENT_ID) {
    return false;
  }

  if (clientSecret && clientSecret !== env.OIDC_CLIENT_SECRET) {
    return false;
  }

  const url = new URL(redirectUri);
  if (
    ((url.origin === env.WEBSITE_URL || pattern.test(url.origin)) && url.pathname === '/authorize') ||
    (url.protocol === 'typie' && url.pathname === '/auth/callback')
  ) {
    return true;
  }

  return true;
};

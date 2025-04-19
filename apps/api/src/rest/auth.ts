import dayjs, { Dayjs } from 'dayjs';
import { and, eq, gt } from 'drizzle-orm';
import escape from 'escape-string-regexp';
import { Hono } from 'hono';
import { deleteCookie, getCookie } from 'hono/cookie';
import * as jose from 'jose';
import { nanoid } from 'nanoid';
import qs from 'query-string';
import { base64 } from 'rfc4648';
import { redis } from '@/cache';
import { db, first, UserAccessTokens, UserSessions } from '@/db';
import { env } from '@/env';
import { decode } from '@/utils';
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
  });
});

auth.get('/jwks', async (c) => {
  const exported = await jose.exportJWK(publicKey);
  return c.json({
    keys: [{ ...exported, kid: jwk.kid, alg: jwk.alg }],
  });
});

auth.get('/authorize', async (c) => {
  const { client_id, redirect_uri, response_type, scope, state, prompt } = c.req.query();

  if (!client_id || !redirect_uri || !response_type) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  if (response_type !== 'code') {
    return c.json({ error: 'unsupported_response_type' }, 400);
  }

  if (!validateClient({ clientId: client_id, redirectUri: redirect_uri })) {
    return c.json({ error: 'invalid_client' }, 400);
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
  const { grant_type, code, redirect_uri, client_id, client_secret } = await c.req.parseBody<Record<string, string>>();

  if (!client_id) {
    return c.json({ error: 'invalid_client' }, 401);
  }

  if (!validateClient({ clientId: client_id, clientSecret: client_secret, redirectUri: redirect_uri })) {
    return c.json({ error: 'invalid_client' }, 401);
  }

  if (grant_type === 'authorization_code') {
    if (!code || !redirect_uri) {
      return c.json({ error: 'invalid_request' }, 400);
    }

    const result = await redis.getdel(`auth:code:${code}`);
    if (!result) {
      return c.json({ error: 'invalid_grant' }, 400);
    }

    const authCode = JSON.parse(result) as AuthorizationCode;

    if (authCode.clientId !== client_id || authCode.redirectUri !== redirect_uri) {
      return c.json({ error: 'invalid_grant' }, 400);
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
      return c.json({ error: 'invalid_grant' }, 400);
    }

    const expiresIn = dayjs.duration(1, 'year');
    const expiresAt = dayjs().add(expiresIn);

    const { accessToken, idToken } = await createTokens({
      clientId: client_id,
      userId: session.userId,
      scope: authCode.scope,
      expiresAt,
    });

    await db.insert(UserAccessTokens).values({
      userId: session.userId,
      sessionId: session.id,
      clientId: client_id,
      token: accessToken,
      scope: authCode.scope,
      expiresAt,
    });

    return c.json({
      access_token: accessToken,
      token_type: 'Bearer',
      expires_in: expiresIn.asSeconds(),
      scope: authCode.scope,
      id_token: idToken,
    });
  }

  return c.json({ error: 'unsupported_grant_type' }, 400);
});

auth.get('/userinfo', async (c) => {
  const authorization = c.req.header('Authorization');
  const accessToken = authorization?.match(/^Bearer\s+(.+)$/)?.[1];

  if (!accessToken) {
    return c.json({ error: 'invalid_token' }, 401);
  }

  try {
    const { payload } = await jose.jwtVerify(accessToken, publicKey);

    if (!payload.sub) {
      return c.json({ error: 'invalid_token' }, 401);
    }

    return c.json({ sub: payload.sub });
  } catch {
    return c.json({ error: 'invalid_token' }, 401);
  }
});

auth.get('/logout', async (c) => {
  const { redirect_uri } = c.req.query();

  if (!redirect_uri) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const token = getCookie(c, 'typie-st');
  if (!token) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const session = await db.select({ id: UserSessions.id }).from(UserSessions).where(eq(UserSessions.token, token)).then(first);

  if (!session) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  await db.transaction(async (tx) => {
    await tx.delete(UserAccessTokens).where(eq(UserAccessTokens.sessionId, session.id));
    await tx.delete(UserSessions).where(eq(UserSessions.id, session.id));
  });

  deleteCookie(c, 'typie-st');

  return c.redirect(redirect_uri);
});

type AuthorizationCode = {
  sessionId: string;
  clientId: string;
  redirectUri: string;
  scope: string;
};

type ValidateClientParams = {
  clientId: string;
  clientSecret?: string;
  redirectUri?: string;
};

const pattern = new RegExp(`^${escape(env.USERSITE_URL).replace(String.raw`\*\.`, String.raw`(([^.]+)\.)?`)}$`);
const validateClient = ({ clientId, clientSecret, redirectUri }: ValidateClientParams) => {
  if (clientId !== env.OIDC_CLIENT_ID) {
    return false;
  }

  if (clientSecret && clientSecret !== env.OIDC_CLIENT_SECRET) {
    return false;
  }

  if (redirectUri) {
    const url = new URL(redirectUri);
    if ((url.origin === env.WEBSITE_URL || pattern.test(url.origin)) && url.pathname === '/authorize') {
      return true;
    }
  }

  return true;
};

type CreateTokensParams = {
  clientId: string;
  userId: string;
  scope: string;
  expiresAt: Dayjs;
};

const createTokens = async ({ clientId, userId, scope, expiresAt }: CreateTokensParams) => {
  const now = dayjs().unix();

  const accessToken = await new jose.SignJWT({
    iss: env.AUTH_URL,
    sub: userId,
    aud: clientId,
    exp: expiresAt.unix(),
    iat: now,
    scope,
  })
    .setProtectedHeader({ alg: jwk.alg as string, kid: jwk.kid })
    .sign(privateKey);

  let idToken;

  if (scope.includes('openid')) {
    idToken = await new jose.SignJWT({
      iss: env.AUTH_URL,
      sub: userId,
      aud: clientId,
      exp: expiresAt.unix(),
      iat: now,
    })
      .setProtectedHeader({ alg: jwk.alg as string, kid: jwk.kid })
      .sign(privateKey);
  }

  return { accessToken, idToken };
};

const jwk = JSON.parse(decode(base64.parse(env.OIDC_JWK))) as jose.JWK;
const publicJwk = { kid: jwk.kid, kty: jwk.kty, alg: jwk.alg, crv: jwk.crv, x: jwk.x };

const privateKey = await jose.importJWK(jwk, jwk.alg);
const publicKey = await jose.importJWK(publicJwk, jwk.alg);

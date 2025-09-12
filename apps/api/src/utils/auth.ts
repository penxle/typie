import * as jose from 'jose';
import { env } from '@/env';
import { decode } from './text';

export const jwk = JSON.parse(
  decode(Uint8Array.fromBase64(env.OIDC_JWK, { alphabet: 'base64url', lastChunkHandling: 'loose' })),
) as jose.JWK;
const publicJwk = { kid: jwk.kid, kty: jwk.kty, alg: jwk.alg, crv: jwk.crv, x: jwk.x };

export const privateKey = await jose.importJWK(jwk, jwk.alg);
export const publicKey = await jose.importJWK(publicJwk, jwk.alg);

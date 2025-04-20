import * as jose from 'jose';
import { base64url } from 'rfc4648';
import { env } from '@/env';
import { decode } from './text';

export const jwk = JSON.parse(decode(base64url.parse(env.OIDC_JWK, { loose: true }))) as jose.JWK;
const publicJwk = { kid: jwk.kid, kty: jwk.kty, alg: jwk.alg, crv: jwk.crv, x: jwk.x };

export const privateKey = await jose.importJWK(jwk, jwk.alg);
export const publicKey = await jose.importJWK(publicJwk, jwk.alg);

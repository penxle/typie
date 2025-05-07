#!/usr/bin/env node

import * as jose from 'jose';
import { base64url } from 'rfc4648';

if (!process.argv[2]) {
  console.error('Usage: node scripts/generate-jwk.ts <kid>');
  process.exit(1);
}

const alg = 'EdDSA';
const { privateKey } = await jose.generateKeyPair(alg, { extractable: true });

const jwk = await jose.exportJWK(privateKey);

jwk.alg = alg;
jwk.kid = process.argv[2];

console.log('JWK representation:');
console.log(JSON.stringify(jwk, undefined, 2));

console.log();
console.log('Environment variable representation:');
console.log(base64url.stringify(new TextEncoder().encode(JSON.stringify(jwk)), { pad: false }));

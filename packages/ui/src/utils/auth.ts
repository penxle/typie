import { decode, encode } from './text';

export const serializeOAuthState = (state: unknown) => {
  return encode(JSON.stringify(state)).toBase64({ alphabet: 'base64url', omitPadding: true });
};

export const deserializeOAuthState = (state: string) => {
  return JSON.parse(decode(Uint8Array.fromBase64(state, { alphabet: 'base64url', lastChunkHandling: 'loose' })));
};

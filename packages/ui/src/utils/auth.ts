import { base64url } from 'rfc4648';
import { decode, encode } from './text';

export const serializeOAuthState = (state: unknown) => {
  return base64url.stringify(encode(JSON.stringify(state)), { pad: false });
};

export const deserializeOAuthState = (state: string) => {
  return JSON.parse(decode(base64url.parse(state, { loose: true })));
};

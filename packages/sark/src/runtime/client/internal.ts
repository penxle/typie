// @ts-expect-error internal import
import factory from '$graphql/client';
import type { SarkClient } from './client';

const client = factory();
export const getClient = (): SarkClient => {
  if (globalThis.window === undefined) {
    return factory();
  }

  return client;
};

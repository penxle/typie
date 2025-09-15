import { createCache } from './cache';
import { RootFieldKey } from './types';
import { makeFieldKeyWithArgs } from './utils';
import type { StorageKey } from './types';

const cache = createCache();

type InvalidateTarget =
  | { __typename: string; id: string | number }
  | { __typename: string; id: string | number; field: string; args?: Record<string, unknown> }
  | { __typename: 'Query' }
  | { __typename: 'Query'; field: string; args?: Record<string, unknown> };

export const cacheOperations = {
  invalidate(...targets: InvalidateTarget[]) {
    for (const target of targets) {
      if (target.__typename === 'Query') {
        if ('field' in target) {
          const fieldKey = makeFieldKeyWithArgs(target.field, target.args);
          cache.invalidate(RootFieldKey, fieldKey);
        } else {
          cache.invalidate(RootFieldKey);
        }
      } else if ('field' in target && 'id' in target) {
        const storageKey = `${target.__typename}:${target.id}` as StorageKey;
        const fieldKey = makeFieldKeyWithArgs(target.field, target.args);
        cache.invalidate(storageKey, fieldKey);
      } else if ('id' in target) {
        const storageKey = `${target.__typename}:${target.id}` as StorageKey;
        cache.invalidate(storageKey);
      }
    }
  },

  clear() {
    cache.clear();
  },
};

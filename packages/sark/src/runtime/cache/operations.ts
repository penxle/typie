import { createCache } from './cache';
import type { EntityKey, FieldKey } from './types';

const cache = createCache();

type InvalidateTarget =
  | string
  | { __typename: string; id: string | number }
  | { __typename: string; id: string | number; fields: string | string[] };

export const cacheOperations = {
  invalidate(...targets: InvalidateTarget[]) {
    for (const target of targets) {
      if (typeof target === 'string') {
        cache.invalidate(target as EntityKey);
      } else {
        const entityKey = `${target.__typename}:${target.id}` as EntityKey;

        if ('fields' in target) {
          const fields = Array.isArray(target.fields) ? target.fields : [target.fields];
          for (const field of fields) {
            cache.invalidate(entityKey, field as FieldKey);
          }
        } else {
          cache.invalidate(entityKey);
        }
      }
    }
  },

  clear() {
    cache.clear();
  },
};

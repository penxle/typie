import { createCache } from './cache';
import { RootFieldKey } from './types';
import { makeFieldKeyWithArgs } from './utils';
import type { QueryKey, StorageKey } from './types';

const cache = createCache();

type InvalidateTarget =
  | { __typename: string; id: string | number }
  | { __typename: string; id: string | number; field: string; args?: Record<string, unknown> }
  | { __typename: 'Query' }
  | { __typename: 'Query'; field: string; args?: Record<string, unknown> };

export const cacheOperations = {
  async invalidate(...targets: InvalidateTarget[]): Promise<void> {
    const allAffectedQueries = new Set<QueryKey>();

    for (const target of targets) {
      let affectedQueries: Set<QueryKey>;

      if (target.__typename === 'Query') {
        if ('field' in target) {
          const fieldKey = makeFieldKeyWithArgs(target.field, target.args);
          affectedQueries = cache.invalidate(RootFieldKey, fieldKey);
        } else {
          affectedQueries = cache.invalidate(RootFieldKey);
        }
      } else if ('field' in target && 'id' in target) {
        const storageKey = `${target.__typename}:${target.id}` as StorageKey;
        const fieldKey = makeFieldKeyWithArgs(target.field, target.args);
        affectedQueries = cache.invalidate(storageKey, fieldKey);
      } else if ('id' in target) {
        const storageKey = `${target.__typename}:${target.id}` as StorageKey;
        affectedQueries = cache.invalidate(storageKey);
      } else {
        affectedQueries = new Set();
      }

      for (const queryKey of affectedQueries) {
        allAffectedQueries.add(queryKey);
      }
    }

    await cache.waitForRefetches(allAffectedQueries);
  },

  clear() {
    cache.clear();
  },
};

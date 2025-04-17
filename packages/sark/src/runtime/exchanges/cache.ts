import { delay, filter, map, merge, mergeMap, never, pipe, share, takeUntil, tap } from 'wonka';
import { createCache } from '../cache/cache';
import { addOperationMeta } from './utils';
import type { Exchange, GraphQLOperation } from '../types';

export const cacheExchange = (): Exchange => {
  return ({ forward }) => {
    return (ops$) => {
      const cache = createCache();

      const cache$ = pipe(
        ops$,
        filter(
          (operation): operation is GraphQLOperation => operation.type === 'query' && operation.context.requestPolicy !== 'network-only',
        ),
        mergeMap((operation) => {
          const teardown$ = pipe(
            ops$,
            filter((op) => op.type === 'teardown' && op.key === operation.key),
          );

          return pipe(
            cache.observe(operation.schema, operation.variables ?? {}),
            delay(0),
            map((v) => ({ operation, ...v })),
            takeUntil(teardown$),
          );
        }),
        share,
      );

      const nonCache$ = pipe(
        ops$,
        filter((operation) => operation.type !== 'query' || operation.context.requestPolicy === 'network-only'),
      );

      const cacheHit$ = pipe(
        cache$,
        filter((result) => !result.partial),
        map((result) => ({
          type: 'data' as const,
          operation: addOperationMeta(result.operation, { cacheOutcome: 'hit' }),
          data: result.data,
        })),
      );

      const cacheMiss$ = pipe(
        cache$,
        filter((result) => result.partial && result.operation.context.requestPolicy !== 'cache-only'),
        map((result) => addOperationMeta(result.operation, { cacheOutcome: 'miss' })),
      );

      const cacheError$ = pipe(
        cache$,
        filter((result) => result.partial && result.operation.context.requestPolicy === 'cache-only'),
        mergeMap(() => never),
      );

      const forward$ = pipe(
        merge([nonCache$, cacheMiss$]),
        forward,
        tap((result) => {
          if (result.type === 'data') {
            cache.writeQuery(result.operation.schema, result.operation.variables ?? {}, result.data);
          }
        }),
      );

      return merge([cacheHit$, cacheError$, forward$]);
    };
  };
};

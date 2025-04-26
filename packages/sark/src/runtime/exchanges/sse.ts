import { createClient } from 'graphql-sse';
import { filter, make, merge, mergeMap, pipe, takeUntil } from 'wonka';
import { GraphQLError, NetworkError } from '../types';
import type { ClientOptions } from 'graphql-sse';
import type { Exchange, GraphQLOperation, OperationResult } from '../types';

export const sseExchange = (url: string, options?: Omit<ClientOptions, 'url'>): Exchange => {
  return ({ forward }) => {
    return (ops$) => {
      const client = createClient({ url, ...options });

      const forward$ = pipe(
        ops$,
        filter((operation) => operation.type === 'teardown' || operation.context.transport !== 'sse'),
        forward,
      );

      const subscription$ = pipe(
        ops$,
        filter((operation): operation is GraphQLOperation => operation.type !== 'teardown' && operation.context.transport === 'sse'),
        mergeMap((operation) => {
          const subscription$ = make<OperationResult>((observer) => {
            return client.subscribe(
              { operationName: operation.schema.name, query: operation.schema.source, variables: operation.variables },
              {
                next: (response) => {
                  if (response.errors && response.errors.length > 0) {
                    observer.next({
                      type: 'error' as const,
                      operation,
                      error: new GraphQLError({
                        message: response.errors.map((e) => e.message).join(', '),
                        path: response.errors[0].path,
                        extensions: response.errors[0].extensions,
                      }),
                    });
                  } else {
                    observer.next({
                      type: 'data' as const,
                      operation,
                      data: response.data as never,
                    });
                  }
                },
                error: (error) => {
                  observer.next({
                    type: 'error' as const,
                    operation,
                    error: new NetworkError({
                      message: error instanceof Error ? error.message : String(error),
                    }),
                  });

                  observer.complete();
                },
                complete: () => {
                  observer.complete();
                },
              },
            );
          });

          const teardown$ = pipe(
            ops$,
            filter((op) => op.type === 'teardown' && op.key === operation.key),
          );

          return pipe(subscription$, takeUntil(teardown$));
        }),
      );

      return merge([forward$, subscription$]);
    };
  };
};

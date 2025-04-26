import { createClient } from 'graphql-ws';
import { nanoid } from 'nanoid';
import { match, P } from 'ts-pattern';
import { filter, make, merge, mergeMap, pipe, takeUntil } from 'wonka';
import { GraphQLError, NetworkError } from '../types';
import type { ClientOptions } from 'graphql-ws';
import type { Exchange, GraphQLOperation, OperationResult } from '../types';

export const wsExchange = (url: string, options?: Omit<ClientOptions, 'url'>): Exchange => {
  return ({ forward }) => {
    return (ops$) => {
      let socket: WebSocket | undefined;
      let timer: NodeJS.Timeout | undefined;

      const client = createClient({
        url,
        generateID: () => nanoid(),
        shouldRetry: () => true,
        retryAttempts: Infinity,
        keepAlive: 10_000,
        on: {
          connected: (_socket) => {
            socket = _socket as WebSocket;
          },
          ping: (received) => {
            if (!received)
              timer = setTimeout(() => {
                if (socket?.readyState === WebSocket.OPEN) {
                  socket.close(4408, 'Request Timeout');
                }
              }, 5000);
          },
          pong: (received) => {
            if (received && timer) {
              clearTimeout(timer);
              timer = undefined;
            }
          },
          closed: () => {
            socket = undefined;

            if (timer) {
              clearTimeout(timer);
              timer = undefined;
            }
          },
        },
        ...options,
      });

      const forward$ = pipe(
        ops$,
        filter((operation) => operation.type === 'teardown' || operation.context.transport !== 'ws'),
        forward,
      );

      const subscription$ = pipe(
        ops$,
        filter((operation): operation is GraphQLOperation => operation.type !== 'teardown' && operation.context.transport === 'ws'),
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
                  const message = match(error)
                    .with(P.instanceOf(CloseEvent), (e) => e.reason)
                    .with(P.instanceOf(Error), (e) => e.message)
                    .otherwise(String);

                  observer.next({
                    type: 'error' as const,
                    operation,
                    error: new NetworkError({
                      message,
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

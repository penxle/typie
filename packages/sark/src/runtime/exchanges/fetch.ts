import { filter, fromAsyncIterable, merge, mergeMap, pipe, takeUntil } from 'wonka';
import { GraphQLError, NetworkError } from '../types';
import type { Exchange, GraphQLOperation, OperationResult } from '../types';

type GraphQLResponse = {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data?: any;
  errors?: { message: string; path?: readonly (string | number)[]; extensions?: Record<string, unknown> }[];
};

async function* makeRequest(operation: GraphQLOperation): AsyncIterable<OperationResult> {
  const fetchFn = operation.context.fetch ?? globalThis.fetch;

  try {
    const resp = await fetchFn(operation.context.url, {
      ...operation.context.fetchOptions,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...operation.context.fetchOptions?.headers,
      },
      body: JSON.stringify({
        operationName: operation.schema.name,
        query: operation.schema.source,
        variables: operation.variables,
      }),
    });

    if (!resp.ok) {
      throw new NetworkError({
        message: `네트워크 오류: ${resp.status} ${resp.statusText}`,
        statusCode: resp.status,
      });
    }

    const result = (await resp.json()) as GraphQLResponse;

    if (result.errors && result.errors.length > 0) {
      throw new GraphQLError({
        message: result.errors.map((e) => e.message).join(', '),
        path: result.errors[0].path,
        extensions: result.errors[0].extensions,
      });
    }

    yield { type: 'data', operation, data: result.data };
  } catch (err: unknown) {
    const error =
      err instanceof NetworkError || err instanceof GraphQLError
        ? err
        : new NetworkError({ message: err instanceof Error ? err.message : String(err) });
    yield { type: 'error', operation, error };
  }
}

export const fetchExchange = (): Exchange => {
  return ({ forward }) => {
    return (ops$) => {
      const forward$ = pipe(
        ops$,
        filter(
          (operation) => (operation.type !== 'query' && operation.type !== 'mutation') || operation.context.requestPolicy === 'cache-only',
        ),
        forward,
      );

      const fetch$ = pipe(
        ops$,
        filter(
          (operation): operation is GraphQLOperation =>
            (operation.type === 'query' || operation.type === 'mutation') && operation.context.requestPolicy !== 'cache-only',
        ),
        mergeMap((operation) => {
          const iter = makeRequest(operation);
          const teardown$ = pipe(
            ops$,
            filter((op) => op.type === 'teardown' && op.key === operation.key),
          );

          return pipe(fromAsyncIterable(iter), takeUntil(teardown$));
        }),
      );

      return merge([forward$, fetch$]);
    };
  };
};

import { fromValue, map, pipe, share, take, toPromise } from 'wonka';
import { NetworkError } from '../../types';
import type { Exchange, ExchangeIO, GraphQLOperation, OperationContext, OperationResult } from '../../types';

type RunExchangeOptions = {
  exchange: Exchange;
  operation: GraphQLOperation;
  result?: OperationResult;
};

export const runExchange = async ({ exchange, operation, result }: RunExchangeOptions): Promise<OperationResult> => {
  const forward: ExchangeIO = (ops$) => {
    if (result) {
      return pipe(
        ops$,
        map(() => ({ ...result, operation })),
      );
    } else {
      return pipe(
        ops$,
        map(() => ({ operation, type: 'error' as const, error: new NetworkError({ message: 'No result' }) })),
      );
    }
  };

  return await pipe(fromValue(operation), exchange({ forward: (ops$) => share(forward(share(ops$))) }), take(1), toPromise);
};

type CreateOperationOptions = {
  url?: string;
  name: string;
  kind: 'query' | 'mutation' | 'subscription';
  source: string;
  variables: Record<string, unknown>;
  context?: Partial<OperationContext>;
};

export const createOperation = ({ url, name, kind, source, variables, context }: CreateOperationOptions): GraphQLOperation => ({
  key: '1',
  type: kind,
  schema: {
    name,
    kind,
    source,
    selections: { operation: [], fragments: {} },
    meta: {},
  },
  variables,
  context: {
    url: url ?? 'https://example.com/graphql',
    requestPolicy: 'cache-first',
    ...context,
  },
});

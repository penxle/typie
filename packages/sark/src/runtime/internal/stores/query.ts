import { concat, fromValue, makeSubject, pipe, subscribe, switchMap, take, toPromise } from 'wonka';
import { getClient } from '../../client/internal';
import type { Readable, Subscriber } from 'svelte/store';
import type { $ArtifactSchema, ArtifactSchema, IsEmpty } from '../../../types';
import type { OperationContext } from '../../types';

type IsClientQuery<T extends $ArtifactSchema<'query'>> = T['$meta']['client'] extends 'true' ? true : false;

export type QueryStore<T extends $ArtifactSchema<'query'>> = Readable<
  IsClientQuery<T> extends true ? T['$output'] | undefined : T['$output']
> & {
  load: IsEmpty<T['$input']> extends true
    ? (variables?: null, context?: Partial<OperationContext>) => Promise<T['$output']>
    : (variables: T['$input'], context?: Partial<OperationContext>) => Promise<T['$output']>;
};

export function createQueryStore<T extends $ArtifactSchema<'query'>>(schema: ArtifactSchema<T>): QueryStore<T> {
  const client = getClient();

  let data: T['$output'] | undefined;
  let variables: T['$input'] = {};
  let context: Partial<OperationContext> | undefined;

  const variables$ = makeSubject<T['$input']>();

  return {
    load: async (_variables?: T['$input'] | null, _context?: Partial<OperationContext>) => {
      variables = _variables ?? {};
      context = _context;

      variables$.next(variables);

      const operation = client.createOperation({
        schema,
        variables,
        context: {
          transport: 'fetch',
          ...context,
          requestPolicy: 'network-only',
        },
      });

      const result$ = client.executeOperation(operation);
      const result = await pipe(result$, take(1), toPromise);

      if (result.type === 'data') {
        data = result.data;
        return data;
      } else {
        throw result.error;
      }
    },

    subscribe: (run: Subscriber<T['$output'] | undefined>) => {
      const isClientQuery = schema.meta.client === 'true'; // @client
      const isDataLoaded = data !== undefined;

      if (!isDataLoaded && !isClientQuery) {
        throw new Error('Data is not loaded');
      }

      run(data);

      // NOTE: @client 쿼리이고 data가 없으면 load 호출 대기 (초기 실행 건너뜀)
      const shouldSkipInitialExecution = !isDataLoaded && isClientQuery;
      const source = shouldSkipInitialExecution ? variables$.source : concat([fromValue(variables), variables$.source]);

      const subscription = pipe(
        source,
        switchMap((variables) => {
          const operation = client.createOperation({
            schema,
            variables,
            context: {
              transport: 'fetch',
              ...context,
              requestPolicy: 'cache-first',
            },
          });

          return client.executeOperation(operation);
        }),
        subscribe((result) => {
          if (result.type === 'data') {
            data = result.data as T['$output'];
            run(data);
          }
        }),
      );

      return () => {
        subscription.unsubscribe();
      };
    },
  } as QueryStore<T>;
}

import { pipe, subscribe, take, toPromise } from 'wonka';
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

  return {
    load: async (_variables?: T['$input'] | null, _context?: Partial<OperationContext>) => {
      variables = _variables ?? {};
      context = _context;

      const operation = client.createOperation({
        schema,
        variables,
        context: {
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
      if (data === undefined && schema.meta.client !== 'true') {
        throw new Error('Data is not loaded');
      }

      run(data);

      const operation = client.createOperation({
        schema,
        variables,
        context: {
          ...context,
          requestPolicy: 'cache-only',
        },
      });

      const result$ = client.executeOperation(operation);

      const subscription = pipe(
        result$,
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

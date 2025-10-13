import { nanoid } from 'nanoid';
import { pipe, take, toPromise } from 'wonka';
import { createCache } from '../../cache/cache';
import { getClient } from '../../client/internal';
import type { $ArtifactSchema, ArtifactSchema, IsEmpty } from '../../../types';
import type { Data } from '../../cache/types';
import type { OperationContext } from '../../types';

type Input<T> = T extends { input: infer U } ? U : never;

export type MutationStore<T extends $ArtifactSchema<'mutation'>> =
  IsEmpty<T['$input']> extends true
    ? (input?: null, context?: Partial<OperationContext>) => Promise<T['$output'][keyof T['$output']]>
    : (input: Input<T['$input']>, context?: Partial<OperationContext>) => Promise<T['$output'][keyof T['$output']]>;

export function createMutationStore<T extends $ArtifactSchema<'mutation'>>(schema: ArtifactSchema<T>): MutationStore<T> {
  const client = getClient();
  const cache = createCache();

  return (async (input: IsEmpty<T['$input']> extends true ? null : Input<T['$input']> | null, context?: Partial<OperationContext>) => {
    const variables = input ? { input } : {};
    const optimisticKey = context?.optimistic ? nanoid() : null;

    if (optimisticKey && context?.optimistic) {
      const rootField = schema.selections.operation[0];
      const mutationFieldName = rootField && 'name' in rootField ? rootField.name : schema.name;
      const optimisticData = { [mutationFieldName]: context.optimistic };
      cache.addOptimisticLayer(optimisticKey, schema, variables, optimisticData as Data);
    }

    const operation = client.createOperation({
      schema,
      variables,
      context: {
        transport: 'fetch',
        ...context,
        requestPolicy: 'network-only',
      },
    });

    try {
      const result$ = client.executeOperation(operation);
      const result = await pipe(result$, take(1), toPromise);

      if (result.type === 'data') {
        if (optimisticKey) {
          cache.removeOptimisticLayer(optimisticKey);
        }
        return Object.values(result.data)[0];
      } else {
        if (optimisticKey) {
          cache.removeOptimisticLayer(optimisticKey);
        }
        throw result.error;
      }
    } catch (err) {
      if (optimisticKey) {
        cache.removeOptimisticLayer(optimisticKey);
      }
      throw err;
    }
  }) as MutationStore<T>;
}

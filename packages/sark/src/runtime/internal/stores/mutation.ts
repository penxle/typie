import { pipe, take, toPromise } from 'wonka';
import { getClient } from '../../client/internal';
import type { $ArtifactSchema, ArtifactSchema, IsEmpty } from '../../../types';
import type { OperationContext } from '../../types';

type Input<T> = T extends { input: infer U } ? U : never;

export type MutationStore<T extends $ArtifactSchema<'mutation'>> =
  IsEmpty<T['$input']> extends true
    ? (input?: null, context?: Partial<OperationContext>) => Promise<T['$output'][keyof T['$output']]>
    : (input: Input<T['$input']>, context?: Partial<OperationContext>) => Promise<T['$output'][keyof T['$output']]>;

export function createMutationStore<T extends $ArtifactSchema<'mutation'>>(schema: ArtifactSchema<T>): MutationStore<T> {
  const client = getClient();

  return (async (input: IsEmpty<T['$input']> extends true ? null : Input<T['$input']> | null, context?: Partial<OperationContext>) => {
    const operation = client.createOperation({
      schema,
      variables: input ? { input } : {},
      context: {
        transport: 'fetch',
        ...context,
        requestPolicy: 'network-only',
      },
    });

    const result$ = client.executeOperation(operation);
    const result = await pipe(result$, take(1), toPromise);

    if (result.type === 'data') {
      return Object.values(result.data)[0];
    } else {
      throw result.error;
    }
  }) as MutationStore<T>;
}

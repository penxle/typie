import { pipe, take, toPromise } from 'wonka';
import { getClient } from '../../client/internal';
import type { $ArtifactSchema, ArtifactSchema, IsEmpty } from '../../../types';

export type MutationStore<T extends $ArtifactSchema<'mutation'>> =
  IsEmpty<T['$input']> extends true
    ? () => Promise<T['$output'][keyof T['$output']]>
    : (input: T['$input'] extends { input: infer U } ? U : never) => Promise<T['$output'][keyof T['$output']]>;

export function createMutationStore<T extends $ArtifactSchema<'mutation'>>(schema: ArtifactSchema<T>): MutationStore<T> {
  const client = getClient();

  return (async (input) => {
    const operation = client.createOperation({
      schema,
      variables: input ? { input } : {},
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

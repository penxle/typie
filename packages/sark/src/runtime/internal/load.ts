import { pipe, take, toPromise } from 'wonka';
import { getClient } from '../client/internal';
import type { $ArtifactSchema, ArtifactSchema } from '../../types';
import type { OperationContext } from '../types';

type QueryArtifactSchema = $ArtifactSchema<'query'>;

export const loadQuery = async <T extends QueryArtifactSchema>(
  schema: ArtifactSchema<T>,
  variables: T['$input'],
  context?: Partial<OperationContext>,
) => {
  const client = getClient();

  const operation = client.createOperation({
    schema,
    variables,
    context: {
      ...context,
      requestPolicy: 'network-only',
    },
  });

  const result = await pipe(client.executeOperation(operation), take(1), toPromise);

  if (result.type === 'data') {
    return result.data;
  } else {
    throw result.error;
  }
};

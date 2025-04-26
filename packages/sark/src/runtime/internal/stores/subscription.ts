import { pipe, subscribe } from 'wonka';
import { getClient } from '../../client/internal';
import type { $ArtifactSchema, ArtifactSchema, IsEmpty } from '../../../types';
import type { OperationContext } from '../../types';

export type SubscriptionStore<T extends $ArtifactSchema<'subscription'>> = {
  subscribe: IsEmpty<T['$input']> extends true
    ? (variables?: null, handler?: (data: T['$output'][keyof T['$output']]) => void, context?: Partial<OperationContext>) => () => void
    : (
        variables: T['$input'],
        handler?: (data: T['$output'][keyof T['$output']]) => void,
        context?: Partial<OperationContext>,
      ) => () => void;
};

export function createSubscriptionStore<T extends $ArtifactSchema<'subscription'>>(schema: ArtifactSchema<T>): SubscriptionStore<T> {
  const client = getClient();

  return {
    subscribe: (
      variables?: T['$input'] | null,
      handler?: (data: T['$output'][keyof T['$output']]) => void,
      context?: Partial<OperationContext>,
    ) => {
      const operation = client.createOperation({
        schema,
        variables: variables ?? {},
        context: {
          transport: 'ws',
          ...context,
          requestPolicy: 'network-only',
        },
      });

      const result$ = client.executeOperation(operation);

      const subscription = pipe(
        result$,
        subscribe((result) => {
          if (result.type === 'data') {
            handler?.(Object.values(result.data)[0] as T['$output'][keyof T['$output']]);
          } else if (result.type === 'error') {
            throw result.error;
          }
        }),
      );

      return () => {
        subscription.unsubscribe();
      };
    },
  };
}

import { filter, map, merge, pipe } from 'wonka';
import type { Exchange, OperationError, OperationResult } from '../types';

export const errorExchange = (transform: (error: OperationError) => OperationError): Exchange => {
  return ({ forward }) => {
    return (ops$) => {
      const forward$ = pipe(ops$, forward);

      const error$ = pipe(
        forward$,
        filter((result) => result.type === 'error'),
        map(
          (result) =>
            ({
              type: 'error' as const,
              operation: result.operation,
              error: transform(result.error),
            }) as OperationResult,
        ),
      );

      const data$ = pipe(
        forward$,
        filter((result) => result.type === 'data'),
      );

      return merge([error$, data$]);
    };
  };
};

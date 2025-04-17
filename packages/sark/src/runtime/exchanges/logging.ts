import { pipe, tap } from 'wonka';
import type { Exchange } from '../types';

export const loggingExchange = (): Exchange => {
  return ({ forward }) => {
    return (ops$) => {
      return pipe(
        ops$,
        tap((operation) => {
          if (operation.type !== 'teardown') {
            console.log('[삵] 요청:', {
              key: operation.key,
              type: operation.type,
              name: operation.schema.name,
              variables: operation.variables,
              _meta: operation.context._meta,
            });
          }
        }),
        forward,
        tap((result) => {
          if (result.type === 'data') {
            console.log('[삵] 응답:', {
              key: result.operation.key,
              type: result.operation.type,
              name: result.operation.schema.name,
              _meta: result.operation.context._meta,
            });
          } else {
            console.error('[삵] 오류:', {
              key: result.operation.key,
              type: result.operation.type,
              name: result.operation.schema.name,
              error: result.error,
              _meta: result.operation.context._meta,
            });
          }
        }),
      );
    };
  };
};

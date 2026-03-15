import { map, pipe } from '@mearie/core/stream';
import { isAggregatedError, isGraphQLError } from '@mearie/svelte';
import { FormError } from '@typie/ui/form';
import { TypieError } from '#/errors';
import type { Exchange } from '@mearie/svelte';

export function unwrapError(err: unknown): unknown {
  return isAggregatedError(err) && err.errors.length === 1 ? err.errors[0] : err;
}

export const errorExchange = (): Exchange => {
  return ({ forward }) => ({
    name: 'error',
    io: (ops$) => {
      return pipe(
        ops$,
        forward,
        map((result) => {
          if (!result.errors || result.errors.length === 0) {
            return result;
          }

          return {
            ...result,
            errors: result.errors.map((err) => {
              if (!isGraphQLError(err) || err.extensions?.type !== 'TypieError') {
                return err;
              }

              if (err.extensions.code === 'validation_error') {
                const extra = err.extensions.extra as { field: string; message: string }[];
                for (const { field, message } of extra) {
                  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                  return new FormError(field.split('.').pop()!, message);
                }
              }

              return new TypieError({
                code: err.extensions.code as string,
                message: err.message,
                status: err.extensions.status as number,
                extra: err.extensions.extra,
              });
            }),
          };
        }),
      );
    },
  });
};

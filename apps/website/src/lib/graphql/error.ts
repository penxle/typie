import { map, pipe } from '@mearie/core/stream';
import { isAggregatedError, isGraphQLError } from '@mearie/svelte';
import { TypieError } from '@typie/lib/errors';
import { FormError } from '@typie/ui/form';
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

              // `validation_error`가 언제나 필드 목록을 싣는 것은 아니다(예: blob 예약의 크기·중복 거부).
              // `extra`가 없을 때 그대로 순회하면 이 exchange가 TypeError를 던져 뮤테이션이 정상적으로
              // 거부되지 못한다 — 없으면 아래 일반 TypieError로 떨어뜨린다.
              if (err.extensions.code === 'validation_error' && Array.isArray(err.extensions.extra)) {
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

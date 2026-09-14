import { TypieError } from '@typie/lib/errors';
import { unwrapError } from '$lib/graphql';

export const publicationErrorCode = (err: unknown): string | null => {
  const error = unwrapError(err);
  return error instanceof TypieError ? error.code : null;
};

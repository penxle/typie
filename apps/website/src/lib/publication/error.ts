import { TypieError } from '@typie/lib/errors';
import { unwrapError } from '$lib/graphql';

export const publicationErrorCode = (err: unknown): string | null => {
  const error = unwrapError(err);
  return error instanceof TypieError ? error.code : null;
};

export const publicationErrorDocumentId = (err: unknown): string | null => {
  const error = unwrapError(err);
  if (!(error instanceof TypieError)) return null;
  const extra = error.extra;
  return typeof extra === 'object' && extra !== null && 'documentId' in extra && typeof extra.documentId === 'string'
    ? extra.documentId
    : null;
};

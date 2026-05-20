import { isAggregatedError, isExchangeError, isGraphQLError } from '@mearie/svelte';
import { TypieError } from '@typie/lib/errors';

const PERMANENT_CODES = new Set(['invalid_changeset_payload']);

export function isPermanentSyncError(err: unknown): boolean {
  if (err instanceof TypieError) {
    return PERMANENT_CODES.has(err.code);
  }
  if (!isAggregatedError(err)) return false;
  for (const e of err.errors) {
    if (e instanceof TypieError) {
      if (PERMANENT_CODES.has(e.code)) return true;
    } else if (isGraphQLError(e)) {
      const code = e.extensions?.code;
      if (typeof code === 'string' && PERMANENT_CODES.has(code)) return true;
    } else if (isExchangeError(e, 'http')) {
      const status = e.extensions?.statusCode;
      if (typeof status === 'number' && status >= 400 && status < 500) return true;
    }
  }
  return false;
}

export function describeSyncError(err: unknown): string {
  if (err instanceof TypieError) {
    return `${err.code}: ${err.message}`;
  }
  if (isGraphQLError(err)) {
    const code = typeof err.extensions?.code === 'string' ? `${err.extensions.code}: ` : '';
    return `${code}${err.message}`;
  }
  if (isExchangeError(err, 'http')) {
    return `http ${err.extensions?.statusCode ?? 'error'}: ${err.message}`;
  }
  if (isAggregatedError(err)) {
    return err.errors.map(describeSyncError).join('; ');
  }
  if (err instanceof Error) {
    return err.message;
  }
  return String(err);
}

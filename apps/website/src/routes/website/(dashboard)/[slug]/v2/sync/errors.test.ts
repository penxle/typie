import { TypieError } from '@typie/lib/errors';
import { describe, expect, it } from 'vitest';
import { describeSyncError, isPermanentSyncError } from './errors';

describe('sync errors', () => {
  it('classifies invalid changeset payload as permanent', () => {
    const error = new TypieError({ code: 'invalid_changeset_payload', message: 'invalid_changeset_payload', status: 400 });

    expect(isPermanentSyncError(error)).toBe(true);
  });

  it('does not classify unknown typie errors as permanent', () => {
    const error = new TypieError({ code: 'unexpected_error_dev', message: 'Unexpected error' });

    expect(isPermanentSyncError(error)).toBe(false);
  });

  it('describes typie errors with code and message', () => {
    const error = new TypieError({ code: 'invalid_changeset_payload', message: 'invalid_changeset_payload', status: 400 });

    expect(describeSyncError(error)).toBe('invalid_changeset_payload: invalid_changeset_payload');
  });

  it('describes regular errors with message', () => {
    const error = new Error('Network error');

    expect(describeSyncError(error)).toBe('Network error');
  });
});

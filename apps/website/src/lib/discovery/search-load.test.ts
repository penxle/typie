import { error, redirect } from '@sveltejs/kit';
import { describe, expect, it } from 'vitest';
import { shouldDegradeSearchError } from './search-load.ts';

const thrown = (fn: () => never) => {
  try {
    fn();
  } catch (err) {
    return err;
  }
  throw new Error('unreachable');
};

describe('shouldDegradeSearchError', () => {
  it('500 HttpError는 인라인 오류로 강등한다', () => {
    expect(shouldDegradeSearchError(thrown(() => error(500, { message: 'x', code: 'unexpected_error' })))).toBe(true);
  });

  it('리다이렉트는 강등하지 않는다', () => {
    expect(shouldDegradeSearchError(thrown(() => redirect(302, '/login')))).toBe(false);
  });

  it('500이 아닌 HttpError와 일반 오류는 강등하지 않는다', () => {
    expect(shouldDegradeSearchError(thrown(() => error(503, { message: 'x', code: 'network_error' })))).toBe(false);
    expect(shouldDegradeSearchError(new Error('boom'))).toBe(false);
  });
});

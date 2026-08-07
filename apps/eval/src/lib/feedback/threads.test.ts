import { describe, expect, it } from 'vitest';
import { canClose, canReopen } from './threads.ts';

describe('thread state guards', () => {
  it('open만 닫을 수 있다', () => {
    expect(canClose('open')).toBe(true);
    expect(canClose('closed')).toBe(false);
    expect(canClose('resolved')).toBe(false);
    expect(canClose('withdrawn')).toBe(false);
  });

  it('closed만 다시 열 수 있다', () => {
    expect(canReopen('closed')).toBe(true);
    expect(canReopen('open')).toBe(false);
    expect(canReopen('resolved')).toBe(false);
    expect(canReopen('withdrawn')).toBe(false);
  });
});

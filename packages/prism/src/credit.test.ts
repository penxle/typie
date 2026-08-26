import { describe, expect, it } from 'vitest';
import { quoteReviewCredits } from './credit.ts';

describe('quoteReviewCredits', () => {
  it('high: 5천자 640, 1만자 680', () => {
    expect(quoteReviewCredits('high', 5000)).toBe(640);
    expect(quoteReviewCredits('high', 10_000)).toBe(680);
  });

  it('medium: 1만자 200 / low: 1만자 40', () => {
    expect(quoteReviewCredits('medium', 10_000)).toBe(200);
    expect(quoteReviewCredits('low', 10_000)).toBe(40);
  });

  it('1천자 단위 올림, 1천자 미만은 1천자', () => {
    expect(quoteReviewCredits('low', 1)).toBe(31);
    expect(quoteReviewCredits('low', 0)).toBe(31);
    expect(quoteReviewCredits('low', 1000)).toBe(31);
    expect(quoteReviewCredits('low', 1001)).toBe(32);
  });

  it('비정수·음수 글자 수는 거부한다', () => {
    expect(() => quoteReviewCredits('low', -1)).toThrow();
    expect(() => quoteReviewCredits('low', 1.5)).toThrow();
  });
});

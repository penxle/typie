import { describe, expect, it } from 'vitest';
import { findPrismCreditPack, PRISM_CREDIT_PACKS, quoteReviewCredits } from './credit.ts';

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

describe('PRISM_CREDIT_PACKS', () => {
  it('5종 격자 — TYP-588 §3 값 그대로', () => {
    expect(PRISM_CREDIT_PACKS.map((p) => [p.pack, p.price, p.credits, p.bonus])).toEqual([
      ['P100', 4900, 100, 0],
      ['P330', 14_900, 300, 30],
      ['P690', 29_900, 600, 90],
      ['P1440', 59_900, 1200, 240],
      ['P3000', 119_900, 2400, 600],
    ]);
  });

  it('보너스율 0/10/15/20/25 단조 증가, 실효 단가 단조 감소, 팩5 실효 40.0원', () => {
    const rates = PRISM_CREDIT_PACKS.map((p) => Math.round((p.bonus / p.credits) * 100));
    expect(rates).toEqual([0, 10, 15, 20, 25]);

    const effective = PRISM_CREDIT_PACKS.map((p) => p.price / (p.credits + p.bonus));
    for (let i = 1; i < effective.length; i++) expect(effective[i]).toBeLessThan(effective[i - 1]);
    expect(effective.at(-1)).toBeCloseTo(40, 1);
  });

  it('수량은 전부 정수, findPrismCreditPack은 격자 행을 돌려준다', () => {
    for (const p of PRISM_CREDIT_PACKS) {
      expect(Number.isSafeInteger(p.price) && Number.isSafeInteger(p.credits) && Number.isSafeInteger(p.bonus)).toBe(true);
    }
    expect(findPrismCreditPack('P330')?.credits).toBe(300);
    expect(findPrismCreditPack('P999' as never)).toBeUndefined();
  });
});

import type { PrismReviewTierName } from './review.ts';

export const PRISM_REVIEW_TARIFF: Record<PrismReviewTierName, { base: number; perK: number }> = {
  low: { base: 30, perK: 1 },
  medium: { base: 180, perK: 2 },
  high: { base: 600, perK: 8 },
};

export const quoteReviewCredits = (tier: PrismReviewTierName, characterCount: number): number => {
  if (!Number.isSafeInteger(characterCount) || characterCount < 0) {
    throw new Error(`invalid character count: ${characterCount}`);
  }

  const { base, perK } = PRISM_REVIEW_TARIFF[tier];
  const thousands = Math.max(1, Math.ceil(characterCount / 1000));

  return base + perK * thousands;
};

export type PrismCreditPackName = 'P100' | 'P330' | 'P690' | 'P1440' | 'P3000';
export type PrismCreditPackGrid = {
  readonly pack: PrismCreditPackName;
  readonly price: number;
  readonly credits: number;
  readonly bonus: number;
};

export const PRISM_CREDIT_PACKS: readonly PrismCreditPackGrid[] = [
  { pack: 'P100', price: 4900, credits: 100, bonus: 0 },
  { pack: 'P330', price: 14_900, credits: 300, bonus: 30 },
  { pack: 'P690', price: 29_900, credits: 600, bonus: 90 },
  { pack: 'P1440', price: 59_900, credits: 1200, bonus: 240 },
  { pack: 'P3000', price: 119_900, credits: 2400, bonus: 600 },
];

export const findPrismCreditPack = (pack: PrismCreditPackName): PrismCreditPackGrid | undefined =>
  PRISM_CREDIT_PACKS.find((grid) => grid.pack === pack);

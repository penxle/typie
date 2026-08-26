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

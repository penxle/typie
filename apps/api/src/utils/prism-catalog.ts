import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { prism, PrismApiError } from '#/external/prism.ts';
import { createTtlCache } from './prism-catalog-core.ts';
import type { PrismReviewTierName, ReviewSeedMapping } from '@typie/prism';
import type { PrismCommand } from '#/external/prism-core.ts';

const log = logger.getChild('prism');

const catalog = createTtlCache({ load: () => prism.getCatalog(), ttlMs: 60_000, failureTtlMs: 30_000 });

export const prismCommands = async (): Promise<PrismCommand[] | null> => {
  try {
    const { commands } = await catalog();
    return commands;
  } catch (err) {
    log.warn('prism catalog unavailable: {*}', { error: err });
    if (!(err instanceof PrismApiError) || err.code === 'malformed-response') Sentry.captureException(err);
    return null;
  }
};

const reviewSeeds = createTtlCache({ load: () => prism.getReviewSeeds(), ttlMs: 60_000, failureTtlMs: 30_000 });

// 실패는 던진다 — 시드 없이 이어서를 시작하면 prism이 invalid seed로 실패해 크레딧만 오간다. 확인 단계에서 끊는다
export const prismReviewSeeds = async (tier: PrismReviewTierName): Promise<ReviewSeedMapping[]> => {
  const seeds = await reviewSeeds();
  return seeds[tier];
};

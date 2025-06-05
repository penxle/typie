import type { PlanRules } from '@/db/schemas/json';

export type PlanId = keyof typeof PlanId;
export const PlanId = {
  // spell-checker:disable
  FULL_ACCESS_1MONTH_WITH_BILLING_KEY: 'PL0FULL1MONTHBLK',
  FULL_ACCESS_1YEAR_WITH_BILLING_KEY: 'PL0FULL1YEARBLK',
  FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE: 'PL0FULL1MONTHIAP',
  FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE: 'PL0FULL1YEARIAP',
  // spell-checker:enable
} as const;

export const defaultPlanRules: PlanRules = {
  maxTotalCharacterCount: 16_000,
  maxTotalBlobSize: 20 * 1000 * 1000,
};

export const SUBSCRIPTION_GRACE_DAYS = 7;

import type { PlanRules } from '@/db/schemas/json';

// spell-checker:disable
export type PlanId = keyof typeof PlanId;
export const PlanId = {
  FULL_ACCESS_1MONTH_WITH_BILLING_KEY: 'PL0FL1MBK',
  FULL_ACCESS_1YEAR_WITH_BILLING_KEY: 'PL0FL1YBK',
  FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE: 'PL0FL1MAP',
  FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE: 'PL0FL1YAP',
  FULL_ACCESS_TRIAL: 'PL0FLTRTR',
} as const;
// spell-checker:enable

export const PlanPair = {
  [PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY]: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY,
  [PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY]: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY,
  [PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE]: PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE,
  [PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE]: PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE,
} as const;

export const defaultPlanRules: PlanRules = {
  maxTotalCharacterCount: 200_000,
  maxTotalBlobSize: 100 * 1000 * 1000,
};

export const SUBSCRIPTION_GRACE_DAYS = 7;
export const TRIAL_DURATION_DAYS = 14;

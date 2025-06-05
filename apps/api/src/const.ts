import type { PlanRules } from '@/db/schemas/json';

export type PlanId = keyof typeof PlanId;
export const PlanId = {
  FULL_ACCESS_1MONTH_WITH_BILLING_KEY: 'PL0FL1MBK',
  FULL_ACCESS_1YEAR_WITH_BILLING_KEY: 'PL0FL1YBK',
  FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE: 'PL0FL1MAP',
  FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE: 'PL0FL1YAP',
} as const;

export const defaultPlanRules: PlanRules = {
  maxTotalCharacterCount: 16_000,
  maxTotalBlobSize: 20 * 1000 * 1000,
};

export const SUBSCRIPTION_GRACE_DAYS = 7;

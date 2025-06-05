import type { PlanRules } from '@/db/schemas/json';

export type PlanId = keyof typeof PlanId;
export const PlanId = {
  FULL_ACCESS_1MONTH: 'PL0FULL1MONTH',
  FULL_ACCESS_1YEAR: 'PL0FULL1YEAR',
} as const;

export const defaultPlanRules: PlanRules = {
  maxTotalCharacterCount: 16_000,
  maxTotalBlobSize: 20 * 1000 * 1000,
};

export const SUBSCRIPTION_GRACE_DAYS = 7;

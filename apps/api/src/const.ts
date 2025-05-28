export type PlanId = keyof typeof PlanId;
export const PlanId = {
  PLUS: 'PL0PLUS',
} as const;

export const PLAN_PAYMENT_GRACE_DAYS = 7;

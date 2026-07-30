import { BillingKeyType, PlanInterval } from './enums.ts';

export const supportsPlanInterval = (type: BillingKeyType, interval: PlanInterval): boolean => {
  if (type === BillingKeyType.KAKAOPAY) {
    return interval === PlanInterval.MONTHLY;
  }

  return true;
};

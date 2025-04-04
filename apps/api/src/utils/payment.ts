import dayjs from 'dayjs';
import { match } from 'ts-pattern';
import { UserPlanBillingCycle } from '@/enums';

export const calculatePaymentAmount = (billingCycle: UserPlanBillingCycle, fee: number) => {
  return match(billingCycle)
    .with(UserPlanBillingCycle.MONTHLY, () => fee)
    .with(UserPlanBillingCycle.YEARLY, () => fee * 10)
    .exhaustive();
};

export const getNextPaymentDate = (billingCycle: UserPlanBillingCycle, enrolledAt: dayjs.Dayjs, previousPaymentDate?: dayjs.Dayjs) => {
  const date = previousPaymentDate ?? dayjs.kst();
  const startOfMonth = date.startOf('month').startOf('day');

  const nextCycleMonth = match(billingCycle)
    .with(UserPlanBillingCycle.MONTHLY, () => startOfMonth.add(1, 'month'))
    .with(UserPlanBillingCycle.YEARLY, () => startOfMonth.add(1, 'year'))
    .exhaustive();

  return nextCycleMonth.date(Math.min(enrolledAt.date(), nextCycleMonth.daysInMonth()));
};

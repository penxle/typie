import { PlanInterval } from '@typie/lib/enums';
import dayjs from 'dayjs';

export const LIFETIME_PERIOD_END = dayjs('9999-12-31T00:00:00.000Z');

export const floorToHourKst = (at: dayjs.Dayjs) => {
  return at.kst().startOf('hour');
};

export type ComputeNextPeriodEndParams = {
  periodStartsAt: dayjs.Dayjs;
  interval: PlanInterval;
  billingAnchorAt: dayjs.Dayjs;
};

export const computeNextPeriodEnd = ({ periodStartsAt, interval, billingAnchorAt }: ComputeNextPeriodEndParams) => {
  if (interval !== PlanInterval.MONTHLY && interval !== PlanInterval.YEARLY) {
    throw new Error(`unsupported interval: ${interval}`);
  }

  const anchor = billingAnchorAt.kst();
  const base = periodStartsAt.kst().add(1, interval === PlanInterval.MONTHLY ? 'month' : 'year');

  return base.date(Math.min(anchor.date(), base.daysInMonth())).hour(anchor.hour()).startOf('hour');
};

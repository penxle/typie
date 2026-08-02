import type dayjs from 'dayjs';

export const derivePaymentKey = (lineageId: string, servicePeriodStartsAt: dayjs.Dayjs) => {
  return `${lineageId}_${servicePeriodStartsAt.valueOf()}`;
};

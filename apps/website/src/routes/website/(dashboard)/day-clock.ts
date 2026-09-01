import type { Dayjs } from 'dayjs';

export const delayUntilNextKstDay = (now: Dayjs): number => {
  const kstNow = now.kst();
  return kstNow.add(1, 'day').startOf('day').diff(kstNow);
};

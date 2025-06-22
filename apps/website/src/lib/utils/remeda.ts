import * as R from 'remeda';

export const debounce = <TArgs extends readonly unknown[]>(fn: (...args: TArgs) => void, delay: number): ((...args: TArgs) => void) => {
  const { call } = R.funnel(
    (args: TArgs) => {
      fn(...args);
    },
    {
      reducer: (_, ...args: TArgs) => args,
      minQuietPeriodMs: delay,
      triggerAt: 'end',
    },
  );
  return call;
};

import '@typie/lib/dayjs';

import dayjs from 'dayjs';
import { describe, expect, test } from 'vitest';
import { delayUntilNextKstDay } from './day-clock';

describe('delayUntilNextKstDay', () => {
  test('returns the remaining time until the next KST midnight', () => {
    const now = dayjs('2026-08-31T14:59:59.500Z');

    expect(delayUntilNextKstDay(now)).toBe(500);
  });

  test('uses KST day boundaries for times in another offset', () => {
    const now = dayjs('2026-08-31T00:00:00-07:00');

    expect(delayUntilNextKstDay(now)).toBe(8 * 60 * 60 * 1000);
  });
});

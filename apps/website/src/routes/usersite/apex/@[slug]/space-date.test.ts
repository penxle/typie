import { describe, expect, it } from 'vitest';
import { pickSpaceDate } from './space-date';

describe('pickSpaceDate', () => {
  const dates = { publishedAt: '2026-09-01T00:00:00+09:00', updatedAt: '2026-09-10T00:00:00+09:00' };

  it('picks the published date', () => {
    expect(pickSpaceDate('PUBLISHED_AT', dates)).toBe(dates.publishedAt);
  });

  it('picks the updated date', () => {
    expect(pickSpaceDate('UPDATED_AT', dates)).toBe(dates.updatedAt);
  });

  it('hides the date', () => {
    expect(pickSpaceDate('NONE', dates)).toBeNull();
  });
});

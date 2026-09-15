import { describe, expect, it } from 'vitest';
import { pickSiteDate } from './site-date';

describe('pickSiteDate', () => {
  const dates = { createdAt: '2026-09-01T00:00:00+09:00', updatedAt: '2026-09-10T00:00:00+09:00' };

  it('picks the created date when the site shows the published date', () => {
    expect(pickSiteDate('PUBLISHED_AT', dates)).toBe(dates.createdAt);
  });

  it('picks the updated date', () => {
    expect(pickSiteDate('UPDATED_AT', dates)).toBe(dates.updatedAt);
  });

  it('hides the date', () => {
    expect(pickSiteDate('NONE', dates)).toBeNull();
  });
});

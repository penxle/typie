import type { SiteDateDisplay } from '@typie/lib/enums';

export const pickSiteDate = (dateDisplay: SiteDateDisplay, dates: { createdAt: string; updatedAt: string }): string | null => {
  if (dateDisplay === 'NONE') return null;
  return dateDisplay === 'PUBLISHED_AT' ? dates.createdAt : dates.updatedAt;
};

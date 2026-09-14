import type { SpaceDateDisplay } from '@typie/lib/enums';

export const pickSpaceDate = (dateDisplay: SpaceDateDisplay, dates: { publishedAt: string; updatedAt: string }): string | null => {
  if (dateDisplay === 'NONE') return null;
  return dateDisplay === 'PUBLISHED_AT' ? dates.publishedAt : dates.updatedAt;
};

import type { RecentEditKind, RecentEditRegion } from '@typie/editor-ffi/browser';

export const RECENT_EDIT_WINDOW_MS = 24 * 60 * 60 * 1000;

export type RulerMark = {
  top: number;
  height: number;
  kind: RecentEditKind;
};

const MIN_MARK_HEIGHT = 3;
const DELETED_MARK_HEIGHT = 2;
const MERGE_GAP = 1;

export const computeRulerMarks = (
  regions: RecentEditRegion[],
  pageTops: number[],
  zoom: number,
  scrollHeight: number,
  trackSize: number,
  trackPadding: number,
): RulerMark[] => {
  if (scrollHeight <= 0 || trackSize <= 0) return [];

  const project = (region: RecentEditRegion): RulerMark | null => {
    const pageTop = pageTops[region.page_idx];
    if (pageTop === undefined || Number.isNaN(pageTop)) return null;
    const docY = pageTop + region.y * zoom;
    const top = trackPadding + (docY / scrollHeight) * trackSize;
    if (region.kind === 'deleted') {
      return { top, height: DELETED_MARK_HEIGHT, kind: 'deleted' };
    }
    const height = Math.max(MIN_MARK_HEIGHT, ((region.height * zoom) / scrollHeight) * trackSize);
    return { top, height, kind: region.kind };
  };

  const merged: RulerMark[] = [];
  for (const kind of ['added', 'modified'] as const) {
    const projected = regions
      .filter((r) => r.kind === kind)
      .map((r) => project(r))
      .filter((m): m is RulerMark => m !== null)
      .toSorted((a, b) => a.top - b.top);
    for (const mark of projected) {
      const last = merged.at(-1);
      if (last && last.kind === kind && mark.top <= last.top + last.height + MERGE_GAP) {
        last.height = Math.max(last.height, mark.top + mark.height - last.top);
      } else {
        merged.push({ ...mark });
      }
    }
  }

  const deleted = regions
    .filter((r) => r.kind === 'deleted')
    .map((r) => project(r))
    .filter((m): m is RulerMark => m !== null)
    .toSorted((a, b) => a.top - b.top);

  for (const mark of deleted) {
    for (const bar of merged) {
      const barBottom = bar.top + bar.height;
      if (mark.top < barBottom && mark.top + mark.height > bar.top) {
        mark.top = mark.top + mark.height / 2 <= bar.top + bar.height / 2 ? bar.top - mark.height : barBottom;
        break;
      }
    }
  }

  return [...merged, ...deleted];
};

import type { LinkRect } from '@typie/editor-ffi/browser';

export type LinkTooltipTarget = {
  link: LinkRect;
  page: number;
  anchorRect: { x: number; y: number; width: number; height: number };
};

type RectLike = LinkTooltipTarget['anchorRect'];

// The tooltip anchors to the link's first rect, independent of pointer/selection.
export const pickLinkTooltipAnchorRect = (rects: RectLike[]): RectLike | null => rects[0] ?? null;

// Stable identity for a link occurrence. A LinkRect carries no node id (a link is
// an inline modifier), so identity is (page, href, first-rect origin) — unique per
// occurrence and stable while it stays in place.
export const linkRectKey = (link: LinkRect): string => `${link.page_idx}:${link.href}:${link.rects[0]?.x ?? 0},${link.rects[0]?.y ?? 0}`;

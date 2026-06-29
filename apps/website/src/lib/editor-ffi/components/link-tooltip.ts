import type { LinkRect, ModifierState, PageRect, Selection } from '@typie/editor-ffi/browser';

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

type SelectionTargetOptions = {
  linkRects: LinkRect[];
  modifierStateLink: ModifierState['link'] | undefined;
  selection: Selection | undefined;
  selectionHeadRect: PageRect | null;
};

export const resolveSelectionTarget = ({
  linkRects,
  modifierStateLink,
  selection,
  selectionHeadRect,
}: SelectionTargetOptions): LinkTooltipTarget | undefined => {
  if (!selection || modifierStateLink?.type !== 'uniform') return;

  // The selection is a single uniform link, but the same href can appear several
  // times in the document — pick the occurrence the selection actually points at,
  // so the tooltip lands on the right one (not the first in the document).
  const uniformHref = modifierStateLink.value.href;
  const sameHrefRects = linkRects.filter((rect) => rect.href === uniformHref);
  if (sameHrefRects.length === 0) return;

  const link = pickSelectedLinkRect(sameHrefRects, selectionHeadRect);

  // Anchor to the link's own first rect, independent of the pointer/selection
  // position, so the tooltip stays fixed — matching the hover path.
  const anchorRect = pickLinkTooltipAnchorRect(link.rects);
  if (!anchorRect) return;

  return {
    link,
    page: link.page_idx,
    anchorRect,
  };
};

// Among link rects sharing the selected href, choose the occurrence the selection
// refers to: a rect on the same line as the selection head, else the first.
const pickSelectedLinkRect = (sameHrefRects: LinkRect[], selectionHeadRect: PageRect | null): LinkRect => {
  const lineMatch =
    selectionHeadRect &&
    sameHrefRects.find(
      (rect) =>
        rect.page_idx === selectionHeadRect.page_idx &&
        rect.rects.some(
          (candidate) =>
            selectionHeadRect.rect.y < candidate.y + candidate.height &&
            selectionHeadRect.rect.y + selectionHeadRect.rect.height > candidate.y,
        ),
    );

  return lineMatch ?? sameHrefRects[0];
};

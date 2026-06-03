import type { LinkRect, ModifierState, PageRect, Selection } from '@typie/editor-ffi/browser';

export type LinkTooltipTarget = {
  link: LinkRect;
  page: number;
  anchorRect: { x: number; y: number; width: number; height: number };
};

type RectLike = LinkTooltipTarget['anchorRect'];

// The tooltip anchors to the link's first rect, independent of pointer/selection.
export const pickLinkTooltipAnchorRect = (rects: RectLike[]): RectLike | null => rects[0] ?? null;

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

  const link = pickSelectedLinkRect(sameHrefRects, selection, selectionHeadRect);

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

// Among link rects sharing the selected href, choose the one the selection refers
// to, in priority order:
//   1. a rect whose node is a selection endpoint (exact match),
//   2. a rect on the same line as the selection head,
//   3. the first occurrence in the document (fallback).
const pickSelectedLinkRect = (sameHrefRects: LinkRect[], selection: Selection, selectionHeadRect: PageRect | null): LinkRect => {
  // Prefer the head node over the anchor node (matches caret-led navigation).
  const endpointNodeIds = [selection.head.node_id, selection.anchor.node_id];
  const endpointMatch = endpointNodeIds
    .map((nodeId) => sameHrefRects.find((rect) => rect.node_id === nodeId))
    .find((rect) => rect !== undefined);
  if (endpointMatch) return endpointMatch;

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

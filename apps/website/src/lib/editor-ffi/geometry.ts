import type { ReferenceElement } from '@floating-ui/dom';
import type { PageRect, Selection } from '@typie/editor-ffi/browser';
import type { Editor, EditorSnapshot } from './editor.svelte';
import type { RevealTargetSpan } from './scroll';

export function roundToScale(value: number, scaleFactor: number): number {
  return Math.round(value * scaleFactor) / scaleFactor;
}

type PageSpanOptions = {
  origin?: number;
  displayZoom?: number;
  scaleFactor?: number;
  pageGap?: number;
};

export function resolvePageSpans(
  pageSizes: readonly { height: number }[],
  { origin = 0, displayZoom = 1, scaleFactor = 1, pageGap = 0 }: PageSpanOptions = {},
) {
  let top = origin;
  return pageSizes.map((size, page) => {
    const span = { page, top, bottom: top + roundToScale(size.height * displayZoom, scaleFactor) };
    top = span.bottom + pageGap;
    return span;
  });
}

type CachedPageSpanOptions = Omit<PageSpanOptions, 'origin'>;

type PageSpanCacheEntry = Required<CachedPageSpanOptions> & {
  spans: ReturnType<typeof resolvePageSpans>;
};

const pageSpanCache = new WeakMap<readonly { height: number }[], PageSpanCacheEntry>();

export function resolveCachedPageSpans(pageSizes: readonly { height: number }[], options: CachedPageSpanOptions = {}) {
  const displayZoom = options.displayZoom ?? 1;
  const scaleFactor = options.scaleFactor ?? 1;
  const pageGap = options.pageGap ?? 0;
  const cached = pageSpanCache.get(pageSizes);
  if (cached?.displayZoom === displayZoom && cached.scaleFactor === scaleFactor && cached.pageGap === pageGap) {
    return cached.spans;
  }

  const spans = resolvePageSpans(pageSizes, { displayZoom, scaleFactor, pageGap });
  pageSpanCache.set(pageSizes, { displayZoom, scaleFactor, pageGap, spans });
  return spans;
}

export function resolvePageAtY(
  pages: readonly { page: number; top: number; bottom: number }[],
  pageSizes: readonly { height: number }[],
  contentY: number,
  zoom: number,
): { page: number; y: number } | null {
  if (pages.length === 0 || !Number.isFinite(contentY) || !Number.isFinite(zoom) || zoom <= 0) return null;

  let low = 0;
  let high = pages.length - 1;
  while (low < high) {
    const middle = (low + high) >>> 1;
    if (pages[middle].bottom <= contentY) low = middle + 1;
    else high = middle;
  }

  let span = pages[low];
  let size = pageSizes[span.page];
  if (!size) return null;
  let y = (contentY - span.top) / zoom;
  if (y < 0 && low > 0) {
    const previous = pages[low - 1];
    if (contentY < (previous.bottom + span.top) / 2) {
      span = previous;
      size = pageSizes[span.page];
      if (!size) return null;
      y = size.height;
    } else {
      y = 0;
    }
  }
  return { page: span.page, y: Math.max(0, Math.min(y, size.height)) };
}

export function pageRectsToRevealTargetSpan(
  rects: readonly PageRect[],
  pages: readonly { top: number }[],
  zoom: number,
): RevealTargetSpan | null {
  let targetTop = Infinity;
  let targetBottom = -Infinity;
  for (const { page_idx, rect } of rects) {
    const page = pages[page_idx];
    if (!page) continue;
    targetTop = Math.min(targetTop, page.top + rect.y * zoom);
    targetBottom = Math.max(targetBottom, page.top + (rect.y + rect.height) * zoom);
  }
  return targetTop === Infinity ? null : { targetTop, targetBottom };
}

export function applyMinimumRevealTargetHeight(target: RevealTargetSpan, minimumHeight: number | undefined): RevealTargetSpan {
  if (minimumHeight === undefined || !Number.isFinite(minimumHeight) || minimumHeight <= 0) return target;
  return {
    targetTop: target.targetTop,
    targetBottom: Math.max(target.targetBottom, target.targetTop + minimumHeight),
  };
}

export function isSelectionCollapsed(selection: Selection | undefined): boolean {
  return (
    selection === undefined ||
    (selection.anchor.node === selection.head.node &&
      selection.anchor.offset === selection.head.offset &&
      selection.anchor.affinity === selection.head.affinity)
  );
}

export function selectionHeadRect(snapshot: EditorSnapshot | undefined): PageRect | null {
  const selection = snapshot?.selection;
  const endpoints = snapshot?.selectionEndpoints;
  if (!selection || isSelectionCollapsed(selection)) {
    const cursor = snapshot?.cursor;
    return cursor ? { page_idx: cursor.page_idx, rect: cursor.line } : null;
  }
  if (!endpoints) return null;

  const head = selection.head;
  const to = endpoints.to_position;
  return head.node === to.node && head.offset === to.offset && head.affinity === to.affinity ? endpoints.to : endpoints.from;
}

export function presentedPageElement(editor: Editor, page: number): HTMLDivElement | undefined {
  return editor.published?.frames.has(page) === true ? editor.pageEls[page] : undefined;
}

export function pageRectToClientRect(editor: Editor, { page_idx, rect }: PageRect): DOMRect | null {
  const zoom = editor.safeDisplayZoom();
  const pageEl = presentedPageElement(editor, page_idx);
  if (!pageEl) return null;

  const pageRect = pageEl.getBoundingClientRect();
  return new DOMRect(pageRect.left + rect.x * zoom, pageRect.top + rect.y * zoom, rect.width * zoom, rect.height * zoom);
}

export function pageRectsToClientRects(editor: Editor, rects: PageRect[]): DOMRect[] {
  const out: DOMRect[] = [];

  for (const rect of rects) {
    const clientRect = pageRectToClientRect(editor, rect);
    if (clientRect) out.push(clientRect);
  }

  return out;
}

export function boundingClientRect(rects: Iterable<DOMRect | null | undefined>): DOMRect | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const rect of rects) {
    if (
      !rect ||
      !Number.isFinite(rect.left) ||
      !Number.isFinite(rect.top) ||
      !Number.isFinite(rect.right) ||
      !Number.isFinite(rect.bottom)
    ) {
      continue;
    }
    minX = Math.min(minX, rect.left);
    minY = Math.min(minY, rect.top);
    maxX = Math.max(maxX, rect.right);
    maxY = Math.max(maxY, rect.bottom);
  }

  return minX === Infinity ? null : new DOMRect(minX, minY, maxX - minX, maxY - minY);
}

export function pageRectsToClientRect(editor: Editor, rects: PageRect[]): DOMRect | null {
  return boundingClientRect(pageRectsToClientRects(editor, rects));
}

export function pageRectsToVirtualElement(editor: Editor, rects: PageRect[]): ReferenceElement {
  return {
    getBoundingClientRect: () => boundingClientRect(pageRectsToLayoutClientRects(editor, rects)) ?? new DOMRect(),
    getClientRects: () => pageRectsToLayoutClientRects(editor, rects),
  };
}

function pageRectsToLayoutClientRects(editor: Editor, rects: PageRect[]): DOMRect[] {
  const zoom = editor.safeDisplayZoom();
  const out: DOMRect[] = [];
  for (const { page_idx, rect } of rects) {
    const pageEl = editor.pageEls[page_idx];
    if (!pageEl) continue;
    const pageRect = pageEl.getBoundingClientRect();
    out.push(new DOMRect(pageRect.left + rect.x * zoom, pageRect.top + rect.y * zoom, rect.width * zoom, rect.height * zoom));
  }
  return out;
}

import type { ReferenceElement } from '@floating-ui/dom';
import type { PageRect, Selection } from '@typie/editor-ffi/browser';
import type { Editor, EditorSnapshot } from './editor.svelte';
import type { RevealTargetSpan } from './scroll';

export function roundToScale(value: number, scaleFactor: number): number {
  return Math.round(value * scaleFactor) / scaleFactor;
}

export function resolvePageSpans(
  pageSizes: readonly { height: number }[],
  {
    origin = 0,
    displayZoom = 1,
    scaleFactor = 1,
    pageGap = 0,
  }: { origin?: number; displayZoom?: number; scaleFactor?: number; pageGap?: number } = {},
) {
  let top = origin;
  return pageSizes.map((size, page) => {
    const span = { page, top, bottom: top + roundToScale(size.height * displayZoom, scaleFactor) };
    top = span.bottom + pageGap;
    return span;
  });
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

import type { ReferenceElement } from '@floating-ui/dom';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

export function pageRectToClientRect(editor: Editor, { page_idx, rect }: PageRect): DOMRect | null {
  const zoom = editor.safeDisplayZoom();
  const pageEl = editor.pageEls[page_idx];
  if (!pageEl) return null;

  const pageRect = pageEl.getBoundingClientRect();
  return new DOMRect(pageRect.left + rect.x * zoom, pageRect.top + rect.y * zoom, rect.width * zoom, rect.height * zoom);
}

function pageRectsToClientRects(editor: Editor, rects: PageRect[]): DOMRect[] {
  const out: DOMRect[] = [];

  for (const rect of rects) {
    const clientRect = pageRectToClientRect(editor, rect);
    if (clientRect) out.push(clientRect);
  }

  return out;
}

function boundingClientRect(rects: DOMRect[]): DOMRect {
  if (rects.length === 0) return new DOMRect();

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const rect of rects) {
    minX = Math.min(minX, rect.left);
    minY = Math.min(minY, rect.top);
    maxX = Math.max(maxX, rect.right);
    maxY = Math.max(maxY, rect.bottom);
  }

  return new DOMRect(minX, minY, maxX - minX, maxY - minY);
}

export function pageRectsToVirtualElement(editor: Editor, rects: PageRect[]): ReferenceElement {
  return {
    getBoundingClientRect: () => boundingClientRect(pageRectsToClientRects(editor, rects)),
    getClientRects: () => pageRectsToClientRects(editor, rects),
  };
}

import type { ReferenceElement } from '@floating-ui/dom';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

function pageRectsToClientRects(editor: Editor, rects: PageRect[]): DOMRect[] {
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

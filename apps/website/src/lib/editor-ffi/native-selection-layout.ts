import { positionInRun } from './native-selection';
import type { SelectionLayoutBlock, SelectionTextRun } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

const selectionFonts = new Map<string, Promise<void>>();

export async function loadSelectionFonts(editor: Editor, blocks: readonly SelectionLayoutBlock[]): Promise<void> {
  const fonts = new Map<string, NonNullable<SelectionTextRun['font']>>();
  for (const block of blocks) for (const run of block.runs) if (run.font) fonts.set(run.font.key, run.font);
  await Promise.all(
    [...fonts.values()].map(async (font) => {
      try {
        let pending = selectionFonts.get(font.key);
        if (!pending) {
          // Reuse the engine's resident metrics. No font URL or glyph payload is fetched.
          const bytes = editor.selectionFont(font.family, font.weight);
          if (!bytes) throw new Error('Selection font metrics are unavailable');
          const face = new FontFace(`typie-selection-${font.key}`, bytes, { weight: String(font.weight) });
          pending = face.load().then(() => {
            document.fonts.add(face);
          });
          selectionFonts.set(font.key, pending);
        }
        await pending;
      } catch (err) {
        selectionFonts.delete(font.key);
        // A damaged/unsupported font must not disable selection for the document.
        // This font alone uses the browser fallback and the usual geometry fit.
        console.error(`Could not prepare native selection font ${font.key}`, err);
      }
    }),
  );
}

// Canvas measureText can use different fallback metrics than DOM text (notably
// emoji). Read the actual DOM once per displayed layout, batching reads before
// writes so the number of runs does not cause repeated synchronous layouts.
export function fitSelectionRuns(root: HTMLElement, blocks: readonly SelectionLayoutBlock[], pages: readonly { top: number }[]) {
  const entries = [...root.querySelectorAll<HTMLElement>('[data-selection-run]')].map((span) => {
    const block = blocks[Number(span.closest<HTMLElement>('[data-selection-block]')?.dataset.selectionBlock)];
    return { span, run: block.runs[Number(span.dataset.selectionRun)] };
  });
  for (const { span } of entries) {
    span.style.left = '0';
    span.style.top = '0';
    span.style.marginRight = '0';
    span.style.transform = 'none';
  }
  const rootRect = root.getBoundingClientRect();
  const zoom = rootRect.width / Number.parseFloat(getComputedStyle(root).width);
  const lines = new Map<HTMLElement, SelectionTextRun>();
  for (const { span, run } of entries) {
    if (span.parentElement) lines.set(span.parentElement, run);
  }
  for (const line of lines.keys()) line.style.top = '0';
  // Inline layout rounds each line's height. Correct its accumulated error once,
  // without removing the in-flow boxes native margin hit testing depends on.
  const lineOffsets = [...lines].map(([line, run]) => ({
    line,
    top: (pages[run.page_idx].top - (line.getBoundingClientRect().top - rootRect.top)) / zoom + run.line.y,
  }));
  const measured = entries.map(({ span, run }) => {
    const bounds = span.getBoundingClientRect();
    const range = document.createRange();
    range.selectNodeContents(span);
    const rect = range.getBoundingClientRect();
    const text = rect.height > 0 ? rect : bounds;
    const scaleX = run.text !== '' && text.width > 0 ? (run.rect.width * zoom) / text.width : 1;
    const scaleY = text.height > 0 ? (run.rect.height * zoom) / text.height : 1;
    return {
      span,
      left: -((text.left - bounds.left) / zoom) * scaleX,
      top: run.rect.y - run.line.y - ((text.top - bounds.top) / zoom) * scaleY,
      margin: run.rect.width - bounds.width / zoom,
      scaleX,
      scaleY,
    };
  });
  for (const { line, top } of lineOffsets) line.style.top = `${top}px`;
  for (const { span, left, top, margin, scaleX, scaleY } of measured) {
    span.style.left = `${left}px`;
    span.style.top = `${top}px`;
    span.style.marginRight = `${margin}px`;
    span.style.transform = `scale(${scaleX}, ${scaleY})`;
  }
}

// Adjust only endpoint spans; keep DOM text, Selection offsets and editor selection untouched.
export function observeNativeSelection(root: HTMLElement, editor: Editor, blocks: readonly SelectionLayoutBlock[]): () => void {
  const adjusted = new Map<HTMLElement, string>();
  const restore = () => {
    for (const [span, style] of adjusted) span.style.cssText = style;
    adjusted.clear();
  };
  const measureEndpoint = (node: Node, offset: number) => {
    const span = node.parentElement;
    if (node.nodeType !== Node.TEXT_NODE || !span?.hasAttribute('data-selection-run') || !root.contains(span)) return;
    const blockElement = span.closest<HTMLElement>('[data-selection-block]');
    const block = blocks[Number(blockElement?.dataset.selectionBlock)];
    const run = block?.runs[Number(span.dataset.selectionRun)];
    // Whole-run boundaries already use the fitted engine rectangle. Measuring
    // a collapsed DOM caret here can introduce browser pixel rounding again.
    if (offset === 0 || !run || run.text === '\n' || run.rect.width <= 0 || offset === run.text.length) return;
    const position = positionInRun(block, run, run.text.slice(0, offset));
    const metrics = editor.cursorForPosition(position);
    if (!metrics || metrics.page_idx !== run.page_idx) return;
    const range = root.ownerDocument.createRange();
    range.setStart(node, offset);
    range.collapse(true);
    const zoom = editor.safeDisplayZoom();
    const rootLeft = root.getBoundingClientRect().x;
    const browser = (range.getBoundingClientRect().x - rootLeft) / zoom;
    const bounds = span.getBoundingClientRect();
    return { span, run, browser, engine: metrics.caret.x, left: (bounds.left - rootLeft) / zoom, right: (bounds.right - rootLeft) / zoom };
  };
  const update = () => {
    restore();
    if (editor.protectContent || !editor.isPublished(editor.appliedRevision)) return;
    const selection = root.ownerDocument.getSelection();
    if (!selection || selection.isCollapsed || !selection.rangeCount) return;
    const range = selection.getRangeAt(0);
    const from = measureEndpoint(range.startContainer, range.startOffset);
    const to = measureEndpoint(range.endContainer, range.endOffset);
    const fitSpan = (edge: NonNullable<typeof from>, browserStart: number, browserEnd: number, engineStart: number, engineEnd: number) => {
      if (Math.abs(browserEnd - browserStart) < 0.01) return;
      const ratio = (engineEnd - engineStart) / (browserEnd - browserStart);
      if (!Number.isFinite(ratio) || ratio <= 0) return;
      adjusted.set(edge.span, edge.span.style.cssText);
      const scale = new DOMMatrixReadOnly(getComputedStyle(edge.span).transform);
      edge.span.style.left = `${engineStart - edge.left - (browserStart - edge.left) * ratio}px`;
      edge.span.style.transform = `scale(${scale.a * ratio}, ${scale.d})`;
    };
    if (from && to && from.span === to.span) {
      fitSpan(from, from.browser, to.browser, from.engine, to.engine);
    } else {
      if (from) {
        const farEdge = from.run.rect.x + (from.run.rtl ? 0 : from.run.rect.width);
        fitSpan(from, from.browser, from.run.rtl ? from.left : from.right, from.engine, farEdge);
      }
      if (to) {
        const farEdge = to.run.rect.x + (to.run.rtl ? to.run.rect.width : 0);
        fitSpan(to, to.run.rtl ? to.right : to.left, to.browser, farEdge, to.engine);
      }
    }
  };
  root.ownerDocument.addEventListener('selectionchange', update);
  return () => {
    root.ownerDocument.removeEventListener('selectionchange', update);
    restore();
  };
}

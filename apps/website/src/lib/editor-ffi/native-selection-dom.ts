import type { ExternalElement, Rect, SelectionLayoutBlock, SelectionTextRun } from '@typie/editor-ffi/browser';

type StyledRun = { run: SelectionTextRun; index: number; style: string };
type TextLine = { page: number; rect: Rect; runs: StyledRun[] };
type LineLayout = { left: number; marginTop: number; width: number; height: number; runs: StyledRun[] };
type TextFragment = {
  width: number;
  height: number;
  marginTop: number;
  paddingTop: number;
  paddingRight: number;
  paddingLeft: number;
  lines: LineLayout[];
};
type BlockBounds = { top: number; bottom: number; left: number; right: number };
type BlockLayout = {
  node: SelectionLayoutBlock['node'];
  marginTop: number;
  height: number;
  fragments: TextFragment[];
  external?: { element: ExternalElement; top: number };
};

function selectionLines(block: SelectionLayoutBlock): TextLine[] {
  const lines: TextLine[] = [];
  for (const [index, run] of block.runs.entries()) {
    let line = lines.at(-1);
    if (!line || line.page !== run.page_idx || line.rect.y !== run.line.y) {
      line = { page: run.page_idx, rect: run.line, runs: [] };
      lines.push(line);
    }
    const previous = line.runs.at(-1)?.run;
    const previousRight = previous ? previous.rect.x + previous.rect.width : line.rect.x;
    const family = run.font ? `typie-selection-${run.font.key}` : 'sans-serif';
    const font = `${run.italic ? 'italic ' : ''}${run.weight} ${run.font_size}px ${family}`;
    const style = `margin-left:${run.rect.x - previousRight}px;font:${font};line-height:1;letter-spacing:${run.letter_spacing}px;`;
    line.runs.push({ run, index, style });
  }
  return lines;
}

export function layoutSelectionBlocks(
  blocks: readonly SelectionLayoutBlock[],
  externalElements: readonly ExternalElement[],
  displayPages: readonly { top: number; bottom: number }[],
  zoom: number,
  width: number,
): BlockLayout[] {
  // Everything below uses unscaled document coordinates. Only the layer is zoomed.
  const pages = displayPages.map((page) => ({ top: page.top / zoom, bottom: page.bottom / zoom }));
  const externals = new Map(externalElements.map((element) => [element.node, element]));
  const pageEdges: ({ top: number; bottom: number; firstBlock: number; lastBlock: number } | undefined)[] = [];

  // Measure each block's content and locate the outermost content on each page.
  // Preserve document order, including table cells whose visual order differs.
  const measured = blocks.map((block, index) => {
    const lines = selectionLines(block);
    const external = externals.get(block.node);
    const bounds: BlockBounds = { top: Infinity, bottom: -Infinity, left: Infinity, right: -Infinity };
    const includeRect = (pageIndex: number, rect: Rect) => {
      const top = (pages[pageIndex]?.top ?? 0) + rect.y;
      const bottom = top + rect.height;
      bounds.top = Math.min(bounds.top, top);
      bounds.bottom = Math.max(bounds.bottom, bottom);
      bounds.left = Math.min(bounds.left, rect.x);
      bounds.right = Math.max(bounds.right, rect.x + rect.width);

      const edge = pageEdges[pageIndex];
      if (edge) {
        if (top < edge.top) {
          edge.top = top;
          edge.firstBlock = index;
        }
        if (bottom >= edge.bottom) {
          edge.bottom = bottom;
          edge.lastBlock = index;
        }
      } else {
        pageEdges[pageIndex] = { top, bottom, firstBlock: index, lastBlock: index };
      }
    };
    for (const line of lines) includeRect(line.page, line.rect);
    if (external) includeRect(external.page_idx, external.bounds);
    if (bounds.top === Infinity) Object.assign(bounds, { top: 0, bottom: 0, left: 0, right: width });
    return { node: block.node, lines, external, bounds };
  });

  // Give page margins to that page's first/last block. Without these in-flow
  // areas, native hit testing can jump to another page while dragging in a margin.
  for (const [pageIndex, edge] of pageEdges.entries()) {
    if (!edge) continue;
    const first = measured[edge.firstBlock].bounds;
    const last = measured[edge.lastBlock].bounds;
    first.top = Math.min(first.top, pages[pageIndex]?.top ?? 0);
    last.bottom = Math.max(last.bottom, pages[pageIndex]?.bottom ?? 0);
  }

  // Resolve all relative spacing here; DOM creation needs no geometry or zoom.
  return measured.map(({ node, lines, external, bounds }, index) => ({
    node,
    marginTop: bounds.top - (index ? measured[index - 1].bounds.bottom : 0),
    height: bounds.bottom - bounds.top,
    fragments: textFragments(lines, bounds, pages, width),
    external: external ? { element: external, top: (pages[external.page_idx]?.top ?? 0) - bounds.top } : undefined,
  }));
}

function textFragments(lines: TextLine[], bounds: BlockBounds, pages: readonly { top: number; bottom: number }[], width: number) {
  const fragments: TextFragment[] = [];
  let fragment: TextFragment | undefined;
  let pageIndex = -1;
  let previousFragmentBottom = bounds.top;
  let previousLineBottom = 0;
  const contentWidth = bounds.right - bounds.left;
  const paddingRight = width - bounds.right;
  for (const line of lines) {
    if (!fragment || pageIndex !== line.page) {
      const pageTop = pages[line.page]?.top ?? 0;
      const top = Math.max(bounds.top, pageTop);
      const bottom = Math.min(bounds.bottom, pages[line.page]?.bottom ?? 0);
      const paddingTop = pageTop + line.rect.y - top;
      const marginTop = top - previousFragmentBottom;
      fragment = {
        width,
        height: bottom - top,
        marginTop,
        paddingTop,
        paddingRight,
        paddingLeft: bounds.left,
        lines: [],
      };
      fragments.push(fragment);
      pageIndex = line.page;
      previousFragmentBottom = bottom;
      previousLineBottom = line.rect.y;
    }
    const left = line.rect.x - bounds.left;
    const marginTop = line.rect.y - previousLineBottom;
    fragment.lines.push({
      left,
      marginTop,
      width: contentWidth,
      height: line.rect.height,
      runs: line.runs,
    });
    previousLineBottom = line.rect.y + line.rect.height;
  }
  return fragments;
}

export function renderSelectionText(element: HTMLElement, block: BlockLayout) {
  const document = element.ownerDocument;
  const content = document.createDocumentFragment();
  for (const fragment of block.fragments) {
    const pageElement = document.createElement('span');
    pageElement.className = 'selection-fragment';
    pageElement.style.cssText =
      `width:${fragment.width}px;height:${fragment.height}px;margin-top:${fragment.marginTop}px;` +
      `padding:${fragment.paddingTop}px ${fragment.paddingRight}px 0 ${fragment.paddingLeft}px;`;
    for (const line of fragment.lines) {
      const lineElement = document.createElement('span');
      lineElement.className = 'selection-line';
      lineElement.style.cssText = `left:${line.left}px;margin-top:${line.marginTop}px;width:${line.width}px;height:${line.height}px;`;
      for (const { run, index, style } of line.runs) {
        const span = document.createElement(run.link ? 'a' : 'span');
        span.className = 'selection-text';
        span.dataset.selectionRun = String(index);
        span.style.cssText = style;
        span.dir = run.rtl ? 'rtl' : 'ltr';
        span.draggable = false;
        const href = selectionLink(run.link);
        if (href) {
          span.setAttribute('href', href);
          span.setAttribute('target', '_blank');
          span.setAttribute('rel', 'noopener noreferrer');
        }
        if (run.text) span.textContent = run.text;
        else span.append(document.createElement('br'));
        lineElement.append(span);
      }
      pageElement.append(lineElement);
    }
    content.append(pageElement);
  }
  // Keep the same nodes while scrolling/selecting. Direct DOM also avoids
  // framework anchors and a reactive component/effect for every run.
  element.replaceChildren(content);
}

function selectionLink(href: string | undefined): string | undefined {
  if (!href) return;
  try {
    return ['http:', 'https:', 'mailto:', 'tel:'].includes(new URL(href, window.location.href).protocol) ? href : undefined;
  } catch {
    return;
  }
}

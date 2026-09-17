import type { Position, Selection, SelectionLayoutBlock, SelectionTextRun } from '@typie/editor-ffi/browser';

export type NativeSelectionRange = {
  selection: Selection;
  prefix?: string;
  suffix?: string;
};

// Range offsets count UTF-16 code units; engine positions count Unicode scalars.
export const positionInRun = (block: SelectionLayoutBlock, run: SelectionTextRun, text: string): Position => {
  const boundary = block.trailing_break;
  if (boundary && run.text === '\n' && run.offset === boundary.anchor.offset) return text ? boundary.head : boundary.anchor;
  return { node: block.node, offset: run.offset + [...text].length, affinity: 'downstream' };
};

type SelectedContent = { type: 'range'; start: Position; end: Position } | { type: 'label'; block: SelectionLayoutBlock; text: string };

function intersection(range: Range, element: Node): Range | undefined {
  const part = range.cloneRange();
  part.selectNodeContents(element);
  if (range.compareBoundaryPoints(Range.END_TO_START, part) >= 0 || range.compareBoundaryPoints(Range.START_TO_END, part) <= 0) return;
  if (range.compareBoundaryPoints(Range.START_TO_START, part) > 0) part.setStart(range.startContainer, range.startOffset);
  if (range.compareBoundaryPoints(Range.END_TO_END, part) < 0) part.setEnd(range.endContainer, range.endOffset);
  return part.collapsed ? undefined : part;
}

// Embed HTML is live, selectable content, but its scripts, styles and hidden
// fallback text are not clipboard content. Read only the selected visible text.
function selectedEmbedText(range: Range, element: Element): string {
  const read = (node: Node): string => {
    if (!range.intersectsNode(node)) return '';
    if (node.nodeType === Node.TEXT_NODE) {
      const parent = node.parentElement;
      if (!parent || getComputedStyle(parent).visibility !== 'visible') return '';
      const part = intersection(range, node);
      if (!part || [...part.getClientRects()].every((rect) => rect.width <= 0 || rect.height <= 0)) return '';
      return part.toString();
    }
    if (!(node instanceof Element) || node.matches('script, style, template, noscript')) return '';
    const style = getComputedStyle(node);
    if (style.display === 'none' || style.userSelect === 'none' || style.webkitUserSelect === 'none') return '';
    if (node.tagName === 'BR') return '\n';
    const text = [...node.childNodes].map(read).join('');
    if (!text) return '';
    // Preserve semantic block breaks without turning visual wrapping into lines.
    return ['block', 'flex', 'grid', 'list-item', 'table-row'].includes(style.display) ? `${text.replace(/\n$/, '')}\n` : text;
  };
  return read(element).replace(/\n$/, '');
}

function readSelectedContent(range: Range, element: HTMLElement, blocks: readonly SelectionLayoutBlock[]): SelectedContent | undefined {
  const blockElement = element.closest<HTMLElement>('[data-selection-block]');
  const block = blocks[Number(blockElement?.dataset.selectionBlock)];
  if (!block) return;

  if (element === blockElement && block.runs.length > 0) {
    // An empty paragraph is a position even when the range ends at offset 0,
    // before its <br>. It needs no selected text to preserve that endpoint.
    return { type: 'range', start: block.before, end: block.after };
  }

  const part = intersection(range, element);
  if (!part) return;
  if (Object.hasOwn(element.dataset, 'selectionRun')) {
    const run = block.runs[Number(element.dataset.selectionRun)];
    const prefix = element.ownerDocument.createRange();
    prefix.selectNodeContents(element);
    prefix.setEnd(part.startContainer, part.startOffset);
    const start = positionInRun(block, run, prefix.toString());
    prefix.setEnd(part.endContainer, part.endOffset);
    return { type: 'range', start, end: positionInRun(block, run, prefix.toString()) };
  }

  // A persistent image marks the atom even when its visible pixels are unmounted.
  const atom = element.querySelector('[data-selection-atom]');
  const full = element.ownerDocument.createRange();
  if (atom) full.selectNode(atom);
  else full.selectNodeContents(element);
  const wholeBlock =
    range.compareBoundaryPoints(Range.START_TO_START, full) <= 0 && range.compareBoundaryPoints(Range.END_TO_END, full) >= 0;
  if (wholeBlock) return { type: 'range', start: block.before, end: block.after };

  const text = [...element.querySelectorAll<HTMLElement>('[data-selection-label], [data-embed-html]')]
    .map((label) =>
      Object.hasOwn(label.dataset, 'embedHtml') ? selectedEmbedText(range, label) : (intersection(range, label)?.toString() ?? ''),
    )
    .filter(Boolean)
    .join('\n');
  if (text) return { type: 'label', block, text };
}

export function readNativeSelection(root: HTMLElement, blocks: readonly SelectionLayoutBlock[]): NativeSelectionRange | undefined {
  const selection = root.ownerDocument.getSelection();
  if (!selection || selection.isCollapsed || selection.rangeCount === 0) return;
  // Range is always ordered, including a backwards native selection.
  const range = selection.getRangeAt(0);
  if (!range.intersectsNode(root)) return;

  const isSelectable = (element: HTMLElement) => {
    if (Object.hasOwn(element.dataset, 'selectionRun')) return true;
    if (!Object.hasOwn(element.dataset, 'selectionBlock')) return false;
    const block = blocks[Number(element.dataset.selectionBlock)];
    // Atoms have no runs; empty paragraphs have only empty runs. Both are boundaries.
    return block?.runs.every((run) => run.text === '') ?? false;
  };
  const common = range.commonAncestorContainer;
  let scope = (common.nodeType === Node.ELEMENT_NODE ? common : common.parentElement) as HTMLElement;
  if (!root.contains(scope)) scope = root;
  const containingElement = scope.closest<HTMLElement>('[data-selection-run], [data-selection-block]');
  if (containingElement && root.contains(containingElement) && isSelectable(containingElement)) scope = containingElement;

  // Prune unselected subtrees and read only the first/last selected element.
  // Rust extracts the intervening document content, including collapsed folds.
  const walker = root.ownerDocument.createTreeWalker(scope, NodeFilter.SHOW_ELEMENT, {
    acceptNode: (node) => {
      if (!range.intersectsNode(node)) return NodeFilter.FILTER_REJECT;
      const element = node as HTMLElement;
      if (!isSelectable(element)) return NodeFilter.FILTER_SKIP;
      return readSelectedContent(range, element, blocks) ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT;
    },
  });
  const firstElement = isSelectable(scope) ? scope : (walker.firstChild() as HTMLElement | null);
  walker.currentNode = scope;
  const lastElement = isSelectable(scope) ? scope : (walker.lastChild() as HTMLElement | null);
  if (!firstElement || !lastElement) return;
  const first = readSelectedContent(range, firstElement, blocks);
  const last = firstElement === lastElement ? first : readSelectedContent(range, lastElement, blocks);
  if (!first || !last) return;

  // A partially selected card label stays plain text. Exclude that atom from the
  // engine range, and attach just the selected label at the corresponding edge.
  const anchor = first.type === 'range' ? first.start : first.block.after;
  const head = last.type === 'range' ? last.end : first === last ? anchor : last.block.before;
  return {
    selection: { anchor, head },
    prefix: first.type === 'label' ? first.text : undefined,
    suffix: last !== first && last.type === 'label' ? last.text : undefined,
  };
}

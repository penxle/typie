import { getAppContext } from '@typie/ui/context';
import { CONTINUOUS_PAGE_MARGIN, PAGE_GAP } from './constants';
import { getEditor } from './context';
import { findScroller } from './utils';

export function typewriterPadding(node: HTMLElement, contentPadding: number) {
  const editor = getEditor();
  const app = getAppContext();

  const scroller = findScroller(node);
  let scrollContainerHeight = 0;

  const resizeObserver = new ResizeObserver((entries) => {
    const entry = entries[0];
    if (entry) {
      scrollContainerHeight = entry.contentRect.height;
      updatePadding();
    }
  });
  resizeObserver.observe(scroller);

  function calculatePadding(): number {
    if (!app.preference.current.typewriterEnabled || scrollContainerHeight <= 0) {
      return contentPadding;
    }

    const isPaginated = editor.layout.layoutMode.type === 'paginated';
    const cursorHeight = editor.cursor.bounds?.height ?? 0;

    const totalContentHeight =
      editor.layout.pageHeights.reduce((sum, h) => sum + h, 0) + (isPaginated ? (editor.layout.pageHeights.length - 1) * PAGE_GAP : 0);

    const bounds = editor.cursor.bounds;
    const pageIdx = editor.cursor.pageIdx;
    let cursorTopInDocument = 0;
    if (bounds && pageIdx !== undefined) {
      for (let i = 0; i < pageIdx; i++) {
        cursorTopInDocument += editor.layout.pageHeights[i] ?? 0;
        if (isPaginated) cursorTopInDocument += PAGE_GAP;
      }
      cursorTopInDocument += bounds.y;
      if (!isPaginated) cursorTopInDocument += contentPadding;
    }

    const totalScrollableContentHeight = totalContentHeight + (isPaginated ? 0 : contentPadding);

    const position = app.preference.current.typewriterPosition;
    const availableRange = scrollContainerHeight - cursorHeight;

    const spaceNeededBelowCursorTop = (1 - position) * availableRange + 2 * cursorHeight;
    const contentBelowCursorTop = totalScrollableContentHeight - cursorTopInDocument + CONTINUOUS_PAGE_MARGIN;

    const extraPaddingNeeded = spaceNeededBelowCursorTop - contentBelowCursorTop;
    return Math.max(contentPadding, extraPaddingNeeded);
  }

  function updatePadding() {
    node.style.paddingBottom = `${calculatePadding()}px`;
  }

  updatePadding();

  $effect(() => {
    void editor.cursor.bounds;
    void editor.cursor.pageIdx;
    void editor.layout.pageHeights;
    void editor.layout.layoutMode;
    void app.preference.current.typewriterEnabled;
    void app.preference.current.typewriterPosition;

    updatePadding();
  });

  return {
    update(newContentPadding: number) {
      contentPadding = newContentPadding;
      updatePadding();
    },
    destroy() {
      resizeObserver.disconnect();
    },
  };
}

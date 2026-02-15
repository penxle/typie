import { getAppContext } from '@typie/ui/context';
import { PAGE_GAP } from './constants';
import { getEditorContext } from './context.svelte';

export function setupTypewriter(getTargetEl: () => HTMLElement | undefined, defaultPadding: number) {
  const { editor } = getEditorContext();

  if (editor.readOnly) {
    $effect(() => {
      const el = getTargetEl();
      if (el) {
        el.style.paddingBottom = `${defaultPadding}px`;
      }
    });
    return;
  }

  const app = getAppContext();

  let scrollContainerHeight = $state(0);

  $effect(() => {
    const scroller = editor.scrollContainerEl;
    if (!scroller) {
      return;
    }

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        scrollContainerHeight = entry.contentRect.height;
      }
    });
    resizeObserver.observe(scroller);

    return () => {
      resizeObserver.disconnect();
    };
  });

  function calculatePadding(): number {
    if (!app.preference.current.typewriterEnabled || scrollContainerHeight <= 0) {
      return defaultPadding;
    }

    const isPaginated = editor.layout.layoutMode.type === 'paginated';
    const cursorHeight = editor.cursor.bounds?.height ?? 0;

    const totalContentHeight =
      editor.layout.pages.reduce((sum, p) => sum + p.height, 0) + (isPaginated ? (editor.layout.pages.length - 1) * PAGE_GAP : 0);

    const bounds = editor.cursor.bounds;
    const pageIdx = editor.cursor.pageIdx;
    let cursorTopInDocument = 0;
    if (bounds && pageIdx !== undefined) {
      for (let i = 0; i < pageIdx; i++) {
        cursorTopInDocument += editor.layout.pages[i]?.height ?? 0;
        if (isPaginated) cursorTopInDocument += PAGE_GAP;
      }
      cursorTopInDocument += bounds.y;
      if (!isPaginated) cursorTopInDocument += defaultPadding;
    }

    const totalScrollableContentHeight = totalContentHeight + (isPaginated ? 0 : defaultPadding);

    const position = app.preference.current.typewriterPosition;
    const availableRange = scrollContainerHeight - cursorHeight;

    const spaceNeededBelowCursorTop = (1 - position) * availableRange + cursorHeight;
    const contentBelowCursorTop = totalScrollableContentHeight - cursorTopInDocument;

    const extraPaddingNeeded = spaceNeededBelowCursorTop - contentBelowCursorTop;
    return Math.max(defaultPadding, extraPaddingNeeded);
  }

  $effect(() => {
    void editor.cursor.bounds;
    void editor.cursor.pageIdx;
    void editor.layout.pages;
    void editor.layout.layoutMode;
    void app.preference.current.typewriterEnabled;
    void app.preference.current.typewriterPosition;
    void scrollContainerHeight;

    const el = getTargetEl();
    if (el) {
      el.style.paddingBottom = `${calculatePadding()}px`;
    }
  });

  $effect(() => {
    if (!app.preference.current.typewriterEnabled || app.preference.current.typewriterPosition === undefined) {
      return;
    }

    if (editor.pendingScrollMode !== 'typewriter') {
      return;
    }

    const bounds = editor.cursor.bounds;
    if (!bounds) {
      return;
    }

    const scroller = editor.scrollContainerEl;
    if (!scroller) {
      return;
    }

    const pageIdx = editor.cursor.pageIdx;
    const containerEl = editor.pageContainerEls[pageIdx];
    if (!containerEl) {
      return;
    }

    editor.pendingScrollMode = null;

    const containerRect = containerEl.getBoundingClientRect();
    const cursorTop = containerRect.top + bounds.y;
    const cursorHeight = bounds.height;

    const scrollerRect = scroller.getBoundingClientRect();
    const position = app.preference.current.typewriterPosition;

    const availableRange = scrollerRect.height - cursorHeight;
    const targetY = scrollerRect.top + availableRange * position;
    const delta = cursorTop - targetY;

    if (Math.abs(delta) > 1) {
      scroller.scrollBy({ top: delta, behavior: 'instant' });
    }
  });
}

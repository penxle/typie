import { getAppContext } from '@typie/ui/context';
import { CONTINUOUS_PAGE_MARGIN } from './constants';
import { getEditorContext } from './context.svelte';

export function setupTypewriter(getTargetEl: () => HTMLElement | undefined, defaultPadding: number) {
  const { editor } = getEditorContext();

  if (editor.readOnly) {
    editor.setTypewriterAvailability(false, false);
    $effect(() => {
      const el = getTargetEl();
      if (el) {
        el.style.paddingBottom = `${defaultPadding}px`;
      }
    });
    return;
  }

  const app = getAppContext();
  editor.setTypewriterAvailability(app.preference.current.typewriterEnabled, app.preference.current.typewriterPosition !== undefined);

  let scrollContainerHeight = $state(0);

  const scrollBy = (scroller: HTMLElement, delta: number) => {
    const maxScrollTop = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
    const targetScrollTop = Math.max(0, Math.min(maxScrollTop, scroller.scrollTop + delta));
    if (Math.abs(targetScrollTop - scroller.scrollTop) <= 1) {
      return;
    }

    scroller.scrollTop = targetScrollTop;
  };

  const computeTypewriterScrollMetrics = () => {
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

    const containerRect = containerEl.getBoundingClientRect();
    const cursorTop = containerRect.top + bounds.y;
    const cursorHeight = bounds.height;

    const scrollerRect = scroller.getBoundingClientRect();
    const position = app.preference.current.typewriterPosition ?? 0.5;
    const availableRange = scrollerRect.height - cursorHeight;
    const targetY = scrollerRect.top + availableRange * position;
    const delta = cursorTop - targetY;

    return { scroller, scrollerRect, cursorTop, cursorHeight, delta };
  };

  $effect(() => {
    const enabled = app.preference.current.typewriterEnabled;
    const hasPosition = app.preference.current.typewriterPosition !== undefined;
    editor.setTypewriterAvailability(enabled, hasPosition);
  });

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

    const layoutMode = editor.layout.layoutMode;
    const trailingBottomMargin = layoutMode.type === 'paginated' ? layoutMode.pageMarginBottom : CONTINUOUS_PAGE_MARGIN;
    const cursorHeight = editor.cursor.bounds?.height ?? 0;
    const collapsedSelectionHeight = editor.selection?.collapsed
      ? (editor.selection.headBounds?.bounds.height ?? cursorHeight)
      : cursorHeight;
    const cursorLeading = Math.max(0, collapsedSelectionHeight - cursorHeight);
    const position = app.preference.current.typewriterPosition ?? 0.5;
    const availableRange = scrollContainerHeight - cursorHeight;
    const spaceNeededBelowCursorTop = (1 - position) * availableRange + cursorHeight;
    const intrinsicSpaceBelowLastLine = trailingBottomMargin + cursorHeight + cursorLeading;
    const requiredPadding = spaceNeededBelowCursorTop - intrinsicSpaceBelowLastLine;

    return Math.max(defaultPadding, requiredPadding);
  }

  $effect(() => {
    void editor.cursor.bounds;
    void editor.selection;
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
    if (editor.pendingScrollConsumer !== 'typewriter') {
      return;
    }
    editor.consumePendingScroll('typewriter');
    const metrics = computeTypewriterScrollMetrics();
    if (!metrics || Math.abs(metrics.delta) <= 1) {
      return;
    }
    scrollBy(metrics.scroller, metrics.delta);
  });
}

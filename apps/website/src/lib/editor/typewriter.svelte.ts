import { getAppContext } from '@typie/ui/context';
import { debounce } from '@typie/ui/utils';
import { cubicOut } from 'svelte/easing';
import { Tween } from 'svelte/motion';
import { PAGE_GAP } from './constants';
import { getEditorContext } from './context.svelte';

export function setupTypewriter(getTargetEl: () => HTMLElement | undefined, defaultPadding: number) {
  const TYPEWRITER_SCROLL_DEBOUNCE_MS = 40;
  const CURSOR_VIEWPORT_GUARD_PX = 16;

  const { editor } = getEditorContext();
  const scrollTop = new Tween(0);
  let scrollTweenTarget = $state<HTMLElement>();

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

  const animateScrollBy = (scroller: HTMLElement, delta: number) => {
    const maxScrollTop = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
    const startScrollTop = scroller.scrollTop;
    const targetScrollTop = Math.max(0, Math.min(maxScrollTop, startScrollTop + delta));
    const distance = targetScrollTop - startScrollTop;
    if (Math.abs(distance) <= 1) {
      return;
    }

    const duration = Math.min(180, Math.max(90, Math.abs(distance) * 0.25));
    scrollTweenTarget = scroller;
    void scrollTop.set(startScrollTop, { duration: 0 });
    void scrollTop.set(targetScrollTop, { duration, easing: cubicOut });
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
    const position = app.preference.current.typewriterPosition;

    const availableRange = scrollerRect.height - cursorHeight;
    const targetY = scrollerRect.top + availableRange * position;
    const delta = cursorTop - targetY;

    return { scroller, scrollerRect, cursorTop, cursorHeight, delta };
  };

  const keepCursorInViewport = (metrics: { scroller: HTMLElement; scrollerRect: DOMRect; cursorTop: number; cursorHeight: number }) => {
    const { scroller, scrollerRect, cursorTop, cursorHeight } = metrics;
    const cursorBottom = cursorTop + cursorHeight;
    const safeTop = scrollerRect.top + CURSOR_VIEWPORT_GUARD_PX;
    const safeBottom = scrollerRect.bottom - CURSOR_VIEWPORT_GUARD_PX;

    let nextScrollTop = scroller.scrollTop;
    if (cursorTop < safeTop) {
      nextScrollTop -= safeTop - cursorTop;
    } else if (cursorBottom > safeBottom) {
      nextScrollTop += cursorBottom - safeBottom;
    }

    const maxScrollTop = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
    const clamped = Math.max(0, Math.min(maxScrollTop, nextScrollTop));
    if (Math.abs(clamped - scroller.scrollTop) <= 1) {
      return;
    }

    scroller.scrollTop = clamped;
    scrollTweenTarget = scroller;
    void scrollTop.set(clamped, { duration: 0 });
  };

  const scheduleDebouncedTypewriterScroll = debounce(() => {
    const metrics = computeTypewriterScrollMetrics();
    if (!metrics || Math.abs(metrics.delta) <= 1) {
      return;
    }
    animateScrollBy(metrics.scroller, metrics.delta);
  }, TYPEWRITER_SCROLL_DEBOUNCE_MS);

  $effect(() => {
    const scroller = scrollTweenTarget;
    if (!scroller) {
      return;
    }

    const maxScrollTop = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
    const nextScrollTop = Math.max(0, Math.min(maxScrollTop, scrollTop.current));

    if (Math.abs(scroller.scrollTop - nextScrollTop) > 0.1) {
      scroller.scrollTop = nextScrollTop;
    }
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

    editor.pendingScrollMode = null;
    const metrics = computeTypewriterScrollMetrics();
    if (!metrics) {
      return;
    }

    keepCursorInViewport(metrics);
    scheduleDebouncedTypewriterScroll();
  });
}

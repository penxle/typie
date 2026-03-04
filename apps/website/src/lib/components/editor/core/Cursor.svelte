<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();
  const CURSOR_VIEWPORT_GUARD_PX = 60;

  let element = $state<HTMLDivElement>();
  let prevCursorPos: { x: number; y: number } | null = null;

  function resetAnimation() {
    if (!element) return;

    element.classList.remove('blink');
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        element?.classList.add('blink');
      });
    });
  }

  function keepCursorInViewport(): boolean {
    if (!element) return false;
    if (editor.pointerState >= 2) return false;

    const scroller = editor.scrollContainerEl;
    if (!scroller) return false;

    const scrollerRect = scroller.getBoundingClientRect();
    const cursorRect = element.getBoundingClientRect();

    const scrollerTop = scrollerRect.top + CURSOR_VIEWPORT_GUARD_PX;
    const scrollerBottom = scrollerRect.bottom - CURSOR_VIEWPORT_GUARD_PX;

    let delta = 0;

    if (cursorRect.top < scrollerTop) {
      delta = cursorRect.top - scrollerTop;
    } else if (cursorRect.bottom > scrollerBottom) {
      delta = cursorRect.bottom - scrollerBottom;
    }

    const maxScrollTop = scroller.scrollHeight - scroller.clientHeight;
    const currentScrollTop = scroller.scrollTop;

    if (delta < 0) {
      delta = Math.max(delta, -currentScrollTop);
    } else {
      delta = Math.min(delta, maxScrollTop - currentScrollTop);
    }

    if (delta === 0) {
      return false;
    }

    scroller.scrollBy({ top: delta, behavior: 'instant' });
    return true;
  }

  $effect(() => {
    if (!element) return;

    const { pageIdx, bounds, visible } = editor.cursor;
    const scrollToCursor = editor.pendingScrollConsumer === 'cursor';
    const containerEls = editor.pageContainerEls;
    const inputEl = editor.inputElement;

    if (bounds && containerEls[pageIdx]) {
      containerEls[pageIdx].append(element);

      element.style.visibility = visible && editor.isFocused ? 'visible' : 'hidden';
      element.style.left = `${Math.round(bounds.x)}px`;
      element.style.top = `${bounds.y}px`;
      element.style.height = `${bounds.height}px`;

      if (inputEl && editor.isFocused) {
        const rect = element.getBoundingClientRect();
        inputEl.style.left = `${rect.left}px`;
        inputEl.style.top = `${rect.top + rect.height / 2}px`;
      }

      if (!prevCursorPos || prevCursorPos.x !== bounds.x || prevCursorPos.y !== bounds.y) {
        resetAnimation();
        prevCursorPos = { x: bounds.x, y: bounds.y };
      }

      if (scrollToCursor) {
        requestAnimationFrame(() => {
          if (editor.pendingScrollConsumer !== 'cursor') {
            return;
          }
          const didScroll = keepCursorInViewport();
          if (didScroll) {
            editor.registerCursorAutoScroll('cursor');
          }
          editor.consumePendingScroll('cursor');
        });
      }
    } else {
      element.style.visibility = 'hidden';
      prevCursorPos = null;
    }
  });
</script>

<div
  bind:this={element}
  class={css({
    pointerEvents: 'none',
    backgroundColor: 'text.default',
    position: 'absolute',
    width: '1px',
    animation: 'blink 1s step-end infinite',
  })}
></div>

<style>
  @keyframes -global-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }
</style>

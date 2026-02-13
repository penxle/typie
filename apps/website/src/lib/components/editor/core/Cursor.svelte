<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { cubicOut } from 'svelte/easing';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  let element = $state<HTMLDivElement>();
  let prevCursorPos: { x: number; y: number } | null = null;
  let animationId: number | null = null;

  function resetAnimation() {
    if (!element) return;

    element.classList.remove('blink');
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        element?.classList.add('blink');
      });
    });
  }

  function scrollIntoView(animate: boolean, position = 0.5) {
    if (!element) return;
    if (!animate && editor.pointerState !== 0) return;

    const scroller = editor.scrollContainerEl;
    if (!scroller) return;

    const scrollerRect = scroller.getBoundingClientRect();
    const cursorRect = element.getBoundingClientRect();

    const margin = 40;
    const scrollerTop = scrollerRect.top + margin;
    const scrollerBottom = scrollerRect.bottom - margin;

    let delta = 0;

    if (animate) {
      const cursorTop = cursorRect.top;
      const cursorHeight = cursorRect.height;
      const availableHeight = scrollerRect.height - cursorHeight;
      const targetOffset = scrollerRect.top + availableHeight * position;
      delta = cursorTop - targetOffset;
    } else {
      if (cursorRect.top < scrollerTop) {
        delta = cursorRect.top - scrollerTop;
      } else if (cursorRect.bottom > scrollerBottom) {
        delta = cursorRect.bottom - scrollerBottom;
      }
    }

    const maxScrollTop = scroller.scrollHeight - scroller.clientHeight;
    const currentScrollTop = scroller.scrollTop;

    if (delta < 0) {
      delta = Math.max(delta, -currentScrollTop);
    } else {
      delta = Math.min(delta, maxScrollTop - currentScrollTop);
    }

    if (delta === 0) {
      return;
    }

    if (animate) {
      const startScrollTop = scroller.scrollTop;
      const duration = 150;
      const startTime = performance.now();

      const animateScroll = (currentTime: number) => {
        const elapsed = currentTime - startTime;
        const progress = Math.min(elapsed / duration, 1);
        const eased = cubicOut(progress);

        scroller.scrollTop = startScrollTop + delta * eased;

        if (progress < 1) {
          animationId = requestAnimationFrame(animateScroll);
        } else {
          animationId = null;
        }
      };

      if (animationId) {
        cancelAnimationFrame(animationId);
        animationId = null;
      }

      animationId = requestAnimationFrame(animateScroll);
    } else {
      scroller.scrollBy({ top: delta, behavior: 'instant' });
    }
  }

  $effect(() => {
    if (!element) return;

    const { pageIdx, bounds, visible } = editor.cursor;
    const scrollToCursor = editor.pendingScrollMode === 'auto';
    const containerEls = editor.pageContainerEls;
    const inputEl = editor.inputElement;

    if (bounds && containerEls[pageIdx]) {
      containerEls[pageIdx].append(element);

      element.style.visibility = visible && editor.isFocused ? 'visible' : 'hidden';
      element.style.left = `${bounds.x}px`;
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
          scrollIntoView(false);
        });
        editor.pendingScrollMode = null;
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

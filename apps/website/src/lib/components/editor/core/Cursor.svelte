<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditor } from '$lib/editor/context';
  import { findScroller } from '$lib/editor/utils';

  const editor = getEditor();

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

  function scrollIntoView() {
    if (!element) return;
    if (!editor.isPointerModeIdle) return;

    const scroller = findScroller(element);
    const scrollerRect = scroller.getBoundingClientRect();
    const cursorRect = element.getBoundingClientRect();

    const margin = 40;
    const scrollerTop = scrollerRect.top + margin;
    const scrollerBottom = scrollerRect.bottom - margin;

    if (cursorRect.top < scrollerTop) {
      const delta = cursorRect.top - scrollerTop;
      scroller.scrollBy({ top: delta, behavior: 'instant' });
    } else if (cursorRect.bottom > scrollerBottom) {
      const delta = cursorRect.bottom - scrollerBottom;
      scroller.scrollBy({ top: delta, behavior: 'instant' });
    }
  }

  $effect(() => {
    if (!element) return;

    const { pageIdx, bounds, show } = editor.cursor;
    const containerEls = editor.pageContainerEls;
    const inputEl = editor.inputElement;

    if (editor.isFocused && bounds && containerEls[pageIdx]) {
      containerEls[pageIdx].append(element);

      element.style.visibility = show ? 'visible' : 'hidden';
      element.style.left = `${bounds.x}px`;
      element.style.top = `${bounds.y}px`;
      element.style.height = `${bounds.height}px`;

      if (inputEl) {
        const rect = element.getBoundingClientRect();
        inputEl.style.left = `${rect.left}px`;
        inputEl.style.top = `${rect.top + rect.height / 2}px`;
      }

      if (!prevCursorPos || prevCursorPos.x !== bounds.x || prevCursorPos.y !== bounds.y) {
        resetAnimation();
        prevCursorPos = { x: bounds.x, y: bounds.y };
      }

      scrollIntoView();
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

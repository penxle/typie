import { on } from 'svelte/events';
import getCaretCoordinates from 'textarea-caret';
import type { Action } from 'svelte/action';

export const textAreaScrollPadding: Action<HTMLTextAreaElement> = (element) => {
  $effect(() => {
    const paddingBottom = Number.parseFloat(getComputedStyle(element).lineHeight) * 2;

    const handler = () => {
      const { top, height } = getCaretCoordinates(element, element.selectionEnd);
      const caretBottom = top + height;

      const visibleTop = element.scrollTop;
      const visibleBottom = visibleTop + element.clientHeight;

      if (caretBottom + paddingBottom > visibleBottom) {
        element.scrollTop = Math.min(caretBottom - element.clientHeight + paddingBottom, element.scrollHeight - element.clientHeight);
        return;
      }

      if (top - paddingBottom < visibleTop) {
        element.scrollTop = Math.max(top - paddingBottom, 0);
      }
    };

    const oninput = on(element, 'input', handler);
    const onkeydown = on(element, 'keydown', (e) => {
      if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
        requestAnimationFrame(handler);
      }
    });

    return () => {
      oninput();
      onkeydown();
    };
  });
};

import getCaretCoordinates from 'textarea-caret';
import type { Action } from 'svelte/action';

export type HeaderVerticalNavigationCallbacks = {
  up?: () => void;
  down?: () => void;
};

export type HeaderCaretMeasurement = {
  top: number;
  context: string;
};

export type MeasureHeaderCaret = (element: HTMLTextAreaElement, offset: number) => HeaderCaretMeasurement | null;

const CARET_CONTEXT_PROPERTIES = [
  'direction',
  'box-sizing',
  'width',
  'height',
  'overflow-x',
  'overflow-y',
  'border-top-width',
  'border-right-width',
  'border-bottom-width',
  'border-left-width',
  'border-style',
  'padding-top',
  'padding-right',
  'padding-bottom',
  'padding-left',
  'font-style',
  'font-variant',
  'font-weight',
  'font-stretch',
  'font-size',
  'font-size-adjust',
  'line-height',
  'font-family',
  'text-align',
  'text-transform',
  'text-indent',
  'text-decoration',
  'letter-spacing',
  'word-spacing',
  'tab-size',
  'white-space',
  'overflow-wrap',
  'word-break',
] as const;

const measureTextareaCaret: MeasureHeaderCaret = (element, offset) => {
  try {
    const { top } = getCaretCoordinates(element, offset);
    if (!Number.isFinite(top)) {
      return null;
    }

    const style = getComputedStyle(element);
    return {
      top,
      context: JSON.stringify([
        element.clientWidth,
        element.clientHeight,
        ...CARET_CONTEXT_PROPERTIES.map((property) => style.getPropertyValue(property)),
      ]),
    };
  } catch {
    return null;
  }
};

const SAME_LINE_TOP_TOLERANCE = 0.5;

export const createHeaderVerticalNavigation = (measureCaret: MeasureHeaderCaret = measureTextareaCaret) => {
  let composing = false;
  let destroyed = false;
  let cancelPending: (() => void) | null = null;

  const clearPending = () => {
    const cancel = cancelPending;
    cancelPending = null;
    cancel?.();
  };

  const handleCompositionStart = () => {
    composing = true;
    clearPending();
  };

  const handleCompositionEnd = () => {
    composing = false;
  };

  const handleKeydown = (event: KeyboardEvent, callbacks: HeaderVerticalNavigationCallbacks) => {
    clearPending();

    const element = event.currentTarget;
    if (
      destroyed ||
      composing ||
      event.isComposing ||
      event.shiftKey ||
      event.altKey ||
      event.metaKey ||
      event.ctrlKey ||
      !(element instanceof HTMLTextAreaElement)
    ) {
      return;
    }

    const onExit = event.key === 'ArrowUp' ? callbacks.up : event.key === 'ArrowDown' ? callbacks.down : undefined;
    if (!onExit || element.selectionStart !== element.selectionEnd) {
      return;
    }

    const beforeCaret = measureCaret(element, element.selectionStart);
    if (!beforeCaret || !Number.isFinite(beforeCaret.top)) {
      return;
    }

    const value = element.value;
    let pending = true;
    const removeInterruptListeners = () => {
      element.removeEventListener('blur', clearPending);
      element.ownerDocument.removeEventListener('pointerdown', clearPending, true);
    };
    element.addEventListener('blur', clearPending, { once: true });
    element.ownerDocument.addEventListener('pointerdown', clearPending, { capture: true, once: true });

    const timer = setTimeout(() => {
      if (!pending) {
        return;
      }
      pending = false;
      cancelPending = null;
      removeInterruptListeners();

      if (
        destroyed ||
        composing ||
        element.ownerDocument.activeElement !== element ||
        element.value !== value ||
        element.selectionStart !== element.selectionEnd
      ) {
        return;
      }

      const afterCaret = measureCaret(element, element.selectionStart);
      if (
        !afterCaret ||
        !Number.isFinite(afterCaret.top) ||
        afterCaret.context !== beforeCaret.context ||
        Math.abs(afterCaret.top - beforeCaret.top) > SAME_LINE_TOP_TOLERANCE
      ) {
        return;
      }

      onExit();
    }, 0);
    cancelPending = () => {
      if (!pending) {
        return;
      }
      pending = false;
      clearTimeout(timer);
      removeInterruptListeners();
    };
  };

  const destroy = () => {
    destroyed = true;
    clearPending();
  };

  return { handleCompositionStart, handleCompositionEnd, handleKeydown, destroy };
};

export const headerVerticalNavigation: Action<HTMLTextAreaElement, HeaderVerticalNavigationCallbacks> = (element, initialCallbacks) => {
  const navigation = createHeaderVerticalNavigation();
  let callbacks = initialCallbacks;

  const handleKeydown = (event: KeyboardEvent) => navigation.handleKeydown(event, callbacks);
  const handleCompositionStart = () => navigation.handleCompositionStart();
  const handleCompositionEnd = () => navigation.handleCompositionEnd();

  element.addEventListener('keydown', handleKeydown);
  element.addEventListener('compositionstart', handleCompositionStart);
  element.addEventListener('compositionend', handleCompositionEnd);

  return {
    update(nextCallbacks) {
      callbacks = nextCallbacks;
    },
    destroy() {
      element.removeEventListener('keydown', handleKeydown);
      element.removeEventListener('compositionstart', handleCompositionStart);
      element.removeEventListener('compositionend', handleCompositionEnd);
      navigation.destroy();
    },
  };
};

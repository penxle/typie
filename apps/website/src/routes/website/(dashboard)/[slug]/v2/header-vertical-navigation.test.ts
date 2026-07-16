import getCaretCoordinates from 'textarea-caret';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHeaderVerticalNavigation, headerVerticalNavigation } from './header-vertical-navigation';

vi.mock('textarea-caret', () => ({
  default: vi.fn(() => ({ top: 24, left: 0, height: 16 })),
}));

type DirectionCallbacks = { up?: () => void; down?: () => void };

describe('createHeaderVerticalNavigation', () => {
  let textarea: HTMLTextAreaElement;
  let caret: { top: number; context: string } | null = { top: 24, context: 'stable' };

  beforeEach(() => {
    vi.useFakeTimers();
    textarea = document.createElement('textarea');
    textarea.value = 'alpha beta gamma';
    textarea.setSelectionRange(5, 5);
    document.body.append(textarea);
    textarea.focus();
    caret = { top: 24, context: 'stable' };
  });

  afterEach(() => {
    document.body.replaceChildren();
    vi.useRealTimers();
  });

  function dispatchKeydown(
    navigation: ReturnType<typeof createHeaderVerticalNavigation>,
    key: string,
    callbacks: DirectionCallbacks,
    init: KeyboardEventInit & { isComposing?: boolean } = {},
  ): KeyboardEvent {
    textarea.addEventListener('keydown', (event) => navigation.handleKeydown(event, callbacks), { once: true });
    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
    if (init.isComposing) {
      Object.defineProperty(event, 'isComposing', { value: true });
    }
    textarea.dispatchEvent(event);
    return event;
  }

  function createNavigation() {
    return createHeaderVerticalNavigation(() => caret);
  }

  it('exits once in the next task when native handling leaves the caret unchanged', () => {
    const navigation = createNavigation();
    const down = vi.fn();

    const event = dispatchKeydown(navigation, 'ArrowDown', { down });

    expect(event.defaultPrevented).toBe(false);
    expect(down).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(down).toHaveBeenCalledOnce();
  });

  it('exits when native handling changes selection within the same visual line', () => {
    const navigation = createNavigation();
    const down = vi.fn();

    dispatchKeydown(navigation, 'ArrowDown', { down });
    textarea.setSelectionRange(9, 9);
    vi.runAllTimers();

    expect(down).toHaveBeenCalledOnce();
  });

  it('cancels exit when native handling changes the value', () => {
    const navigation = createNavigation();
    const down = vi.fn();

    dispatchKeydown(navigation, 'ArrowDown', { down });
    textarea.value = 'changed';
    vi.runAllTimers();

    expect(down).not.toHaveBeenCalled();
  });

  it('stays when native handling reaches text end on a different visual line', () => {
    const navigation = createNavigation();
    const down = vi.fn();

    dispatchKeydown(navigation, 'ArrowDown', { down });
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    caret = { top: 48, context: 'stable' };
    vi.runAllTimers();

    expect(down).not.toHaveBeenCalled();
  });

  it('cancels exit when caret measurement is missing non-finite or reflowed', () => {
    const exit = vi.fn();

    caret = null;
    dispatchKeydown(createNavigation(), 'ArrowDown', { down: exit });

    caret = { top: NaN, context: 'stable' };
    dispatchKeydown(createNavigation(), 'ArrowDown', { down: exit });

    caret = { top: 24, context: 'before' };
    dispatchKeydown(createNavigation(), 'ArrowDown', { down: exit });
    caret = { top: 24, context: 'after' };
    vi.runAllTimers();

    expect(exit).not.toHaveBeenCalled();
  });

  it('cancels exit when the textarea loses focus', () => {
    const navigation = createNavigation();
    const down = vi.fn();
    dispatchKeydown(navigation, 'ArrowDown', { down });

    const button = document.createElement('button');
    document.body.append(button);
    button.focus();
    vi.runAllTimers();

    expect(down).not.toHaveBeenCalled();
  });

  it('lets only the latest armed direction exit', () => {
    const navigation = createNavigation();
    const up = vi.fn();
    const down = vi.fn();

    dispatchKeydown(navigation, 'ArrowDown', { down });
    dispatchKeydown(navigation, 'ArrowUp', { up });
    vi.runAllTimers();

    expect(down).not.toHaveBeenCalled();
    expect(up).toHaveBeenCalledOnce();
  });

  it('invalidates an armed exit on a later keydown pointer press or range selection', () => {
    const keyNavigation = createNavigation();
    const keyExit = vi.fn();
    dispatchKeydown(keyNavigation, 'ArrowDown', { down: keyExit });
    dispatchKeydown(keyNavigation, 'a', { down: keyExit });

    const pointerNavigation = createNavigation();
    const pointerExit = vi.fn();
    dispatchKeydown(pointerNavigation, 'ArrowDown', { down: pointerExit });
    textarea.dispatchEvent(new Event('pointerdown', { bubbles: true }));

    const rangeNavigation = createNavigation();
    const rangeExit = vi.fn();
    dispatchKeydown(rangeNavigation, 'ArrowDown', { down: rangeExit });
    textarea.setSelectionRange(2, 5);

    vi.runAllTimers();

    expect(keyExit).not.toHaveBeenCalled();
    expect(pointerExit).not.toHaveBeenCalled();
    expect(rangeExit).not.toHaveBeenCalled();
  });

  it('does not arm modified arrows range selections or active composition', () => {
    const navigation = createNavigation();
    const exit = vi.fn();

    dispatchKeydown(navigation, 'ArrowDown', { down: exit }, { shiftKey: true });
    dispatchKeydown(navigation, 'ArrowDown', { down: exit }, { altKey: true });
    dispatchKeydown(navigation, 'ArrowDown', { down: exit }, { metaKey: true });
    dispatchKeydown(navigation, 'ArrowDown', { down: exit }, { ctrlKey: true });

    textarea.setSelectionRange(2, 5);
    dispatchKeydown(navigation, 'ArrowDown', { down: exit });

    textarea.setSelectionRange(5, 5);
    navigation.handleCompositionStart();
    dispatchKeydown(navigation, 'ArrowDown', { down: exit });
    navigation.handleCompositionEnd();
    dispatchKeydown(navigation, 'ArrowDown', { down: exit }, { isComposing: true });

    vi.runAllTimers();
    expect(exit).not.toHaveBeenCalled();
  });

  it('invalidates an armed exit when composition starts', () => {
    const navigation = createNavigation();
    const down = vi.fn();
    dispatchKeydown(navigation, 'ArrowDown', { down });

    navigation.handleCompositionStart();
    vi.runAllTimers();

    expect(down).not.toHaveBeenCalled();
  });

  it('binds composition and key events and cleans up a pending exit when destroyed', () => {
    const down = vi.fn();
    const getComputedStyleSpy = vi.spyOn(globalThis, 'getComputedStyle').mockReturnValue({
      getPropertyValue: () => 'stable',
    } as unknown as CSSStyleDeclaration);
    const action = headerVerticalNavigation(textarea, { down });

    textarea.dispatchEvent(new Event('compositionstart'));
    textarea.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    vi.runAllTimers();
    expect(down).not.toHaveBeenCalled();

    textarea.dispatchEvent(new Event('compositionend'));
    textarea.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    vi.runAllTimers();
    expect(getCaretCoordinates).toHaveBeenCalledTimes(2);
    expect(down).toHaveBeenCalledOnce();

    textarea.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    action?.destroy?.();
    vi.runAllTimers();
    expect(down).toHaveBeenCalledOnce();
    getComputedStyleSpy.mockRestore();
  });

  it('ignores missing directions disposal and stale timers', () => {
    const navigation = createNavigation();
    const exit = vi.fn();

    dispatchKeydown(navigation, 'ArrowUp', { down: exit });
    dispatchKeydown(navigation, 'ArrowDown', { down: exit });
    navigation.destroy();
    vi.runAllTimers();

    expect(exit).not.toHaveBeenCalled();
  });
});

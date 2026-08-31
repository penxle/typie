import { pushEscapeHandler } from '@typie/ui/utils';
import { describe, expect, it, vi } from 'vitest';

describe('Escape stack', () => {
  it('leaves composing Escape to the native input method', () => {
    const handler = vi.fn(() => true);
    const remove = pushEscapeHandler(handler);
    const event = new KeyboardEvent('keydown', { key: 'Escape', isComposing: true, bubbles: true, cancelable: true });

    try {
      window.dispatchEvent(event);

      expect(handler).not.toHaveBeenCalled();
      expect(event.defaultPrevented).toBe(false);
    } finally {
      remove();
    }
  });

  it('does not run after an earlier handler consumes Escape', () => {
    const handler = vi.fn(() => true);
    const remove = pushEscapeHandler(handler);
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    event.preventDefault();

    try {
      window.dispatchEvent(event);

      expect(handler).not.toHaveBeenCalled();
    } finally {
      remove();
    }
  });

  it('runs only the topmost accepting handler', () => {
    const lower = vi.fn(() => true);
    const upper = vi.fn(() => true);
    const removeLower = pushEscapeHandler(lower);
    const removeUpper = pushEscapeHandler(upper);
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });

    try {
      window.dispatchEvent(event);

      expect(upper).toHaveBeenCalledOnce();
      expect(lower).not.toHaveBeenCalled();
      expect(event.defaultPrevented).toBe(true);
    } finally {
      removeUpper();
      removeLower();
    }
  });
});

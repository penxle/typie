import { describe, expect, it, vi } from 'vitest';
import { handleContextMenu } from './contextmenu';
import type { Editor } from '../editor.svelte';

const createEvent = () =>
  ({
    clientX: 110,
    clientY: 220,
    currentTarget: {} as HTMLElement,
    preventDefault: vi.fn(),
  }) as unknown as MouseEvent & { currentTarget: HTMLElement };

const createEditor = ({ readOnly = false } = {}) =>
  ({
    readOnly,
    gesture: {
      shouldSuppressNativeContextMenu: vi.fn(() => false),
    },
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    interactiveHitTest: vi.fn(),
    enqueue: vi.fn(),
    flush: vi.fn(),
    collectContextMenuContributions: vi.fn(() => []),
    openContextMenu: vi.fn(),
  }) as unknown as Editor & {
    enqueue: ReturnType<typeof vi.fn>;
    flush: ReturnType<typeof vi.fn>;
    openContextMenu: ReturnType<typeof vi.fn>;
  };

describe('handleContextMenu', () => {
  it('sends a secondary pointer down before opening the menu', () => {
    const editor = createEditor();
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'pointer',
      event: {
        type: 'secondary_down',
        page: 0,
        x: 10,
        y: 20,
      },
    });
    expect(editor.flush).toHaveBeenCalledTimes(1);
    expect(editor.flush.mock.invocationCallOrder[0]).toBeLessThan(editor.openContextMenu.mock.invocationCallOrder[0]);
  });

  it('expands the hit word before opening a read-only context menu', () => {
    const editor = createEditor({ readOnly: true });
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).toHaveBeenNthCalledWith(2, {
      type: 'selection',
      op: { type: 'expand', unit: 'word' },
    });
    expect(editor.flush).toHaveBeenCalledTimes(1);
    expect(editor.flush.mock.invocationCallOrder[0]).toBeLessThan(editor.openContextMenu.mock.invocationCallOrder[0]);
  });
});

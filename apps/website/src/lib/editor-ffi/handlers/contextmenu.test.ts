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

const createEditor = ({ readOnly = false, selectionHit = false, isSelectionCollapsed = true } = {}) =>
  ({
    readOnly,
    isSelectionCollapsed,
    gesture: {
      shouldSuppressNativeContextMenu: vi.fn(() => false),
    },
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    interactiveHitTest: vi.fn(),
    selectionHitTest: vi.fn(() => selectionHit),
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
  it('sets selection at the hit point before opening the menu', () => {
    const editor = createEditor();
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: { type: 'set_at', page: 0, x: 10, y: 20 },
    });
    expect(editor.flush).toHaveBeenCalledTimes(1);
    expect(editor.flush.mock.invocationCallOrder[0]).toBeLessThan(editor.openContextMenu.mock.invocationCallOrder[0]);
  });

  it('selects the hit word before opening a read-only context menu', () => {
    const editor = createEditor({ readOnly: true });
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: { type: 'select_unit_at', page: 0, x: 10, y: 20, unit: 'word' },
    });
    expect(editor.flush).toHaveBeenCalledTimes(1);
    expect(editor.flush.mock.invocationCallOrder[0]).toBeLessThan(editor.openContextMenu.mock.invocationCallOrder[0]);
  });

  it('preserves a range selection when opening inside it', () => {
    const editor = createEditor({ selectionHit: true, isSelectionCollapsed: false });
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(editor.flush).toHaveBeenCalledTimes(1);
  });
});

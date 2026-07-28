import { describe, expect, it, vi } from 'vitest';
import { handleContextMenu } from './contextmenu';
import type { Editor } from '../editor.svelte';

const createEvent = () =>
  ({
    clientX: 110,
    clientY: 220,
    currentTarget: {} as HTMLElement,
    preventDefault: vi.fn(),
  }) as unknown as MouseEvent & {
    currentTarget: HTMLElement;
    preventDefault: ReturnType<typeof vi.fn>;
  };

const createEditor = ({
  readOnly = false,
  selectionHit = false,
  isSelectionCollapsed = true,
  appliedSelection,
}: {
  readOnly?: boolean;
  selectionHit?: boolean;
  isSelectionCollapsed?: boolean;
  appliedSelection?: Editor['appliedSnapshot']['selection'];
} = {}) =>
  ({
    readOnly,
    isSelectionCollapsed,
    appliedSnapshot: { selection: appliedSelection },
    gesture: {
      shouldSuppressNativeContextMenu: vi.fn(() => false),
    },
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    interactiveHitTest: vi.fn(),
    selectionHitTest: vi.fn(() => selectionHit),
    enqueue: vi.fn(),
    updateNow: vi.fn((build: () => void) => {
      build();
      return null;
    }),
    collectContextMenuContributions: vi.fn(() => []),
    openContextMenu: vi.fn(),
  }) as unknown as Editor & {
    enqueue: ReturnType<typeof vi.fn>;
    updateNow: ReturnType<typeof vi.fn>;
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
    expect(editor.updateNow).toHaveBeenCalledTimes(1);
    expect(editor.updateNow.mock.invocationCallOrder[0]).toBeLessThan(editor.openContextMenu.mock.invocationCallOrder[0]);
  });

  it('selects the hit word before opening a read-only context menu', () => {
    const editor = createEditor({ readOnly: true });
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: { type: 'select_unit_at', page: 0, x: 10, y: 20, unit: 'word' },
    });
    expect(editor.updateNow).toHaveBeenCalledTimes(1);
    expect(editor.updateNow.mock.invocationCallOrder[0]).toBeLessThan(editor.openContextMenu.mock.invocationCallOrder[0]);
  });

  it('preserves a range selection when opening inside it', () => {
    const editor = createEditor({
      selectionHit: true,
      appliedSelection: {
        anchor: { node: 'text', offset: 0, affinity: 'downstream' },
        head: { node: 'text', offset: 4, affinity: 'downstream' },
      },
    });
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(editor.updateNow).toHaveBeenCalledTimes(1);
  });

  it('suppresses the native menu before applying the hit selection', () => {
    const editor = createEditor();
    const event = createEvent();

    handleContextMenu(editor, event);

    expect(event.preventDefault.mock.invocationCallOrder[0]).toBeLessThan(editor.updateNow.mock.invocationCallOrder[0] ?? 0);
  });
});

import { describe, expect, it, vi } from 'vitest';
import { openLinkEditorFromTooltip } from './link';

const caret = { node: 't1', offset: 0, affinity: 'downstream' as const };
const point = { page: 0, x: 25, y: 25 };

describe('openLinkEditorFromTooltip', () => {
  it('extends the selection over the whole link span and opens the toolbar editor', async () => {
    const span = {
      anchor: { node: 't1', offset: 0, affinity: 'downstream' as const },
      head: { node: 't2', offset: 5, affinity: 'downstream' as const },
    };
    const editor = {
      enqueue: vi.fn(),
      flush: vi.fn(),
      focus: vi.fn(),
      modifierSpanSelection: vi.fn(() => span),
      selection: { anchor: caret, head: caret },
    };
    const ctx = { linkEditorOpen: false };
    const closeTooltip = vi.fn();

    const opened = await openLinkEditorFromTooltip({ closeTooltip, ctx, editor, point });

    expect(opened).toBe(true);
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set_at', page: 0, x: 25, y: 25 } });
    expect(editor.modifierSpanSelection).toHaveBeenCalledWith(caret, 'link');
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set', selection: span } });
    expect(editor.flush).toHaveBeenCalled();
    expect(closeTooltip).toHaveBeenCalled();
    expect(ctx.linkEditorOpen).toBe(true);
  });

  it('falls back to a collapsed caret when the span cannot be resolved', async () => {
    const editor = {
      enqueue: vi.fn(),
      flush: vi.fn(),
      focus: vi.fn(),
      modifierSpanSelection: vi.fn(),
      selection: { anchor: caret, head: caret },
    };
    const ctx = { linkEditorOpen: false };

    await openLinkEditorFromTooltip({ closeTooltip: vi.fn(), ctx, editor, point });

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: { type: 'set', selection: { anchor: caret, head: caret } },
    });
  });

  it('does nothing when the editor instance is unavailable', async () => {
    const ctx = { linkEditorOpen: false };
    const closeTooltip = vi.fn();

    const opened = await openLinkEditorFromTooltip({ closeTooltip, ctx, editor: undefined, point });

    expect(opened).toBe(false);
    expect(closeTooltip).not.toHaveBeenCalled();
    expect(ctx.linkEditorOpen).toBe(false);
  });
});

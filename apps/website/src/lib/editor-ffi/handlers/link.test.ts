import { describe, expect, it, vi } from 'vitest';
import { openLinkEditorFromTooltip } from './link';

describe('openLinkEditorFromTooltip', () => {
  it('extends the selection over the whole link span and opens the toolbar editor', async () => {
    const span = {
      anchor: { node_id: 't1', offset: 0 },
      head: { node_id: 't2', offset: 5 },
    };
    const editor = {
      enqueue: vi.fn(),
      flush: vi.fn(),
      focus: vi.fn(),
      modifierSpanSelection: vi.fn(() => span),
    };
    const ctx = { linkEditorOpen: false };
    const closeTooltip = vi.fn();

    const opened = await openLinkEditorFromTooltip({ closeTooltip, ctx, editor, nodeId: 't1' });

    expect(opened).toBe(true);
    expect(editor.modifierSpanSelection).toHaveBeenCalledWith({ node_id: 't1', offset: 0 }, 'link');
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
    };
    const ctx = { linkEditorOpen: false };

    await openLinkEditorFromTooltip({ closeTooltip: vi.fn(), ctx, editor, nodeId: 't1' });

    const caret = { node_id: 't1', offset: 0 };
    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: { type: 'set', selection: { anchor: caret, head: caret } },
    });
  });

  it('does nothing when the editor instance is unavailable', async () => {
    const ctx = { linkEditorOpen: false };
    const closeTooltip = vi.fn();

    const opened = await openLinkEditorFromTooltip({ closeTooltip, ctx, editor: undefined, nodeId: 't1' });

    expect(opened).toBe(false);
    expect(closeTooltip).not.toHaveBeenCalled();
    expect(ctx.linkEditorOpen).toBe(false);
  });
});

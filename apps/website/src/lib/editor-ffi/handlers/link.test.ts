import { describe, expect, it, vi } from 'vitest';
import { openLinkCardAtPoint } from './link';
import type { MarkCardRequest } from '../editor.svelte';
import type { EditorRequest } from '../editor-update';

const caret = { node: 't1', offset: 0, affinity: 'downstream' as const };
const point = { page: 0, x: 25, y: 25 };
const anchor = { page_idx: 0, rect: { x: 20, y: 20, width: 40, height: 16 } };

const createEditor = (span: { anchor: typeof caret; head: typeof caret } | undefined) => ({
  enqueue: vi.fn(),
  updateNow: vi.fn((build: (request: EditorRequest) => void) => {
    build({} as EditorRequest);
    return {
      revision: 1,
      snapshot: { selection: { anchor: caret, head: caret } },
      commandOutcomes: [{ type: 'applied' as const }],
      events: [],
      awaitPublished: vi.fn(),
    };
  }),
  focus: vi.fn(),
  modifierSpanSelection: vi.fn(() => span),
});

describe('openLinkCardAtPoint', () => {
  it('extends the selection over the whole link span and opens the edit card at the link', () => {
    const span = {
      anchor: { node: 't1', offset: 0, affinity: 'downstream' as const },
      head: { node: 't2', offset: 5, affinity: 'downstream' as const },
    };
    const editor = createEditor(span);
    const ctx: { markCard: MarkCardRequest | null } = { markCard: null };

    const opened = openLinkCardAtPoint({ ctx, editor, point, anchor });

    expect(opened).toBe(true);
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set_at', page: 0, x: 25, y: 25 } });
    expect(editor.modifierSpanSelection).toHaveBeenCalledWith(caret, 'link');
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set', selection: span } });
    expect(editor.updateNow).toHaveBeenCalledTimes(2);
    expect(editor.focus).toHaveBeenCalled();
    expect(ctx.markCard).toEqual({ kind: 'link', mode: 'edit', anchor });
  });

  it('falls back to a collapsed caret when the span cannot be resolved', () => {
    const editor = createEditor(undefined);
    const ctx: { markCard: MarkCardRequest | null } = { markCard: null };

    openLinkCardAtPoint({ ctx, editor, point, anchor });

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: { type: 'set', selection: { anchor: caret, head: caret } },
    });
  });

  it('does nothing when the editor instance is unavailable', () => {
    const ctx: { markCard: MarkCardRequest | null } = { markCard: null };

    const opened = openLinkCardAtPoint({ ctx, editor: undefined, point, anchor });

    expect(opened).toBe(false);
    expect(ctx.markCard).toBeNull();
  });
});

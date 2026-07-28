import { describe, expect, it, vi } from 'vitest';
import { EditorUpdate } from './editor-update';
import type { EditorSnapshot } from './editor.svelte';

describe('EditorUpdate', () => {
  it('keeps the exact applied snapshot and forwards publication cancellation', async () => {
    const snapshot = { revision: 7 } as EditorSnapshot;
    const awaitPublished = vi.fn(async () => ({ type: 'published' as const, revision: 7 }));
    const update = new EditorUpdate(7, snapshot, [], [], awaitPublished);
    const controller = new AbortController();

    await expect(update.awaitPublished(controller.signal)).resolves.toEqual({ type: 'published', revision: 7 });
    expect(update.snapshot).toBe(snapshot);
    expect(awaitPublished).toHaveBeenCalledWith(controller.signal);
  });
});

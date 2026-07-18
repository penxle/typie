import { describe, expect, it, vi } from 'vitest';
import { wireImeResyncListener } from './ime-resync';
import type { Editor } from '../editor.svelte';
import type { ImeInputAdapter } from './ime-input-adapter';

const createFakeEditor = () => {
  const listeners = new Map<string, Set<() => void>>();
  return {
    on: (event: string, cb: () => void): (() => void) => {
      let set = listeners.get(event);
      if (!set) {
        set = new Set();
        listeners.set(event, set);
      }
      set.add(cb);
      return () => {
        set.delete(cb);
      };
    },
    emit: (event: string) => {
      for (const cb of listeners.get(event) ?? []) {
        cb();
      }
    },
  };
};

describe('wireImeResyncListener', () => {
  it('registers, reacts to the event, and unregisters on cleanup', async () => {
    const editor = createFakeEditor();
    const input = document.createElement('textarea');
    const adapter = { resetForResync: vi.fn() };

    const cleanup = wireImeResyncListener(editor as unknown as Editor, adapter as unknown as ImeInputAdapter, () => input);

    editor.emit('ime_resync_required');
    expect(adapter.resetForResync).not.toHaveBeenCalled();

    await Promise.resolve();
    expect(adapter.resetForResync).toHaveBeenCalledOnce();
    expect(adapter.resetForResync).toHaveBeenCalledWith(input);

    cleanup();
    editor.emit('ime_resync_required');
    await Promise.resolve();
    expect(adapter.resetForResync).toHaveBeenCalledOnce();
  });
});

import { afterEach, describe, expect, it, vi } from 'vitest';
import { destroyAll, fanOutResourceUpdate, register, snapshot, unregister } from './registry';
import type { ResourceUpdate } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

afterEach(() => {
  for (const editor of snapshot()) unregister(editor);
  vi.restoreAllMocks();
});

describe('editor registry resource fan-out', () => {
  it('isolates one Host admission failure and releases the shared update once', () => {
    const error = new Error('admission failed');
    const first = {
      receiveResourceUpdate: vi.fn(() => {
        throw error;
      }),
    } as unknown as Editor;
    const second = { receiveResourceUpdate: vi.fn() } as unknown as Editor;
    const update = { free: vi.fn() } as unknown as ResourceUpdate;
    vi.spyOn(console, 'error').mockImplementation(() => false);
    register(first);
    register(second);

    fanOutResourceUpdate(update);

    expect(first.receiveResourceUpdate).toHaveBeenCalledWith(update);
    expect(second.receiveResourceUpdate).toHaveBeenCalledWith(update);
    expect(update.free).toHaveBeenCalledTimes(1);
  });

  it('destroys every live editor from a stable snapshot', () => {
    const first = { destroy: vi.fn() } as unknown as Editor;
    const second = { destroy: vi.fn() } as unknown as Editor;
    vi.mocked(first.destroy).mockImplementation(() => unregister(first));
    vi.mocked(second.destroy).mockImplementation(() => unregister(second));
    register(first);
    register(second);

    destroyAll();

    expect(first.destroy).toHaveBeenCalledOnce();
    expect(second.destroy).toHaveBeenCalledOnce();
    expect(snapshot()).toEqual([]);
  });

  it('continues destroying editors after one fails', () => {
    const error = new Error('destroy failed');
    const first = { destroy: vi.fn() } as unknown as Editor;
    const second = { destroy: vi.fn() } as unknown as Editor;
    vi.mocked(first.destroy).mockImplementation(() => {
      unregister(first);
      throw error;
    });
    vi.mocked(second.destroy).mockImplementation(() => unregister(second));
    vi.spyOn(console, 'error').mockImplementation(() => false);
    register(first);
    register(second);

    destroyAll();

    expect(first.destroy).toHaveBeenCalledOnce();
    expect(second.destroy).toHaveBeenCalledOnce();
    expect(console.error).toHaveBeenCalledWith('Failed to destroy an editor.', error);
  });
});

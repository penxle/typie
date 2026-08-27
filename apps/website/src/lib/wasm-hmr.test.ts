import { describe, expect, test, vi } from 'vitest';
import { registerWasmHmrCleanup } from './wasm-hmr';

describe('WASM HMR cleanup', () => {
  test('cleans up when its module is replaced', () => {
    let dispose: (() => void) | undefined;
    const hot = {
      dispose: vi.fn((callback: () => void) => {
        dispose = callback;
      }),
    };
    const cleanup = vi.fn();

    registerWasmHmrCleanup(hot, cleanup);
    dispose?.();

    expect(cleanup).toHaveBeenCalledOnce();
  });
});

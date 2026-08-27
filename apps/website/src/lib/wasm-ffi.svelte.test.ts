import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  cleanup: undefined as (() => void) | undefined,
  createInstance: vi.fn(),
  destroyAll: vi.fn(),
  host: { free: vi.fn() },
  registerWasmHmrCleanup: vi.fn((_hot: unknown, cleanup: () => void) => {
    mocks.cleanup = cleanup;
  }),
}));

vi.mock('@typie/editor-ffi/browser', () => ({ createInstance: mocks.createInstance }));
vi.mock('@typie/editor-ffi/browser/icu.zst?url', () => ({ default: '/editor-icu.zst' }));
vi.mock('@typie/editor-ffi/browser/wasm?url', () => ({ default: '/editor.wasm' }));
vi.mock('$lib/editor-ffi/registry', () => ({ destroyAll: mocks.destroyAll }));
vi.mock('$lib/wasm-hmr', () => ({ registerWasmHmrCleanup: mocks.registerWasmHmrCleanup }));

const editorModule = () => ({ EditorHost: { create: vi.fn(() => mocks.host) } });

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  mocks.cleanup = undefined;
  mocks.createInstance.mockResolvedValue(editorModule());
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(new Uint8Array([1, 2, 3]))),
  );
  vi.spyOn(WebAssembly, 'compileStreaming').mockResolvedValue({} as WebAssembly.Module);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('Editor WASM HMR lifecycle', () => {
  it('destroys live editors before freeing the initialized host', async () => {
    const { initWasm } = await import('./wasm-ffi.svelte');
    await initWasm();

    mocks.cleanup?.();

    expect(mocks.destroyAll).toHaveBeenCalledOnce();
    expect(mocks.host.free).toHaveBeenCalledOnce();
    expect(mocks.destroyAll.mock.invocationCallOrder[0]).toBeLessThan(mocks.host.free.mock.invocationCallOrder[0] ?? 0);
  });

  it('frees a host that finishes initializing after HMR disposal', async () => {
    const deferredInstance = Promise.withResolvers<ReturnType<typeof editorModule>>();
    mocks.createInstance.mockImplementation(() => deferredInstance.promise);
    const { initWasm } = await import('./wasm-ffi.svelte');
    const initialization = initWasm();
    await vi.waitFor(() => expect(mocks.createInstance).toHaveBeenCalledOnce());

    mocks.cleanup?.();
    deferredInstance.resolve(editorModule());

    await expect(initialization).rejects.toThrow('initialization was canceled');
    expect(mocks.destroyAll).toHaveBeenCalledOnce();
    expect(mocks.host.free).toHaveBeenCalledOnce();
  });
});

import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const mocks = vi.hoisted(() => {
  const PrismWebRuntime = { create: vi.fn() };
  return {
    cleanup: undefined as (() => void) | undefined,
    compileStreaming: vi.fn(async () => ({ compiled: true })),
    createInstance: vi.fn(async () => ({ PrismWebRuntime })),
    fetch: vi.fn(async () => new Response()),
    PrismWebRuntime,
    registerWasmHmrCleanup: vi.fn((_hot: unknown, cleanup: () => void) => {
      mocks.cleanup = cleanup;
    }),
    runtime: { destroy: vi.fn() },
    runtimeOptions: undefined as { loadRenderer: () => Promise<unknown> } | undefined,
  };
});

vi.mock('@typie/prism-ui', () => ({
  createPrismRuntime: (options: typeof mocks.runtimeOptions) => {
    mocks.runtimeOptions = options;
    return mocks.runtime;
  },
}));
vi.mock('@typie/prism-ui-web/browser', () => ({ createInstance: mocks.createInstance }));
vi.mock('@typie/prism-ui-web/browser/wasm?url', () => ({ default: '/prism-ui.wasm' }));
vi.mock('$lib/wasm-hmr', () => ({ registerWasmHmrCleanup: mocks.registerWasmHmrCleanup }));

describe('Typie Prism renderer loader', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    mocks.cleanup = undefined;
    vi.stubGlobal('fetch', mocks.fetch);
    vi.spyOn(WebAssembly, 'compileStreaming').mockImplementation(mocks.compileStreaming);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  test('instantiates the post-processed module with its emitted WASM asset', async () => {
    await import('./runtime.ts');
    const loaded = await mocks.runtimeOptions?.loadRenderer();

    expect(mocks.fetch).toHaveBeenCalledWith('/prism-ui.wasm');
    expect(mocks.compileStreaming).toHaveBeenCalledOnce();
    expect(mocks.createInstance).toHaveBeenCalledWith({ compiled: true });
    expect(loaded).toMatchObject({ PrismWebRuntime: mocks.PrismWebRuntime });
  });

  test('destroys the Prism runtime when the WASM module is replaced', async () => {
    await import('./runtime.ts');

    mocks.cleanup?.();

    expect(mocks.runtime.destroy).toHaveBeenCalledOnce();
  });
});

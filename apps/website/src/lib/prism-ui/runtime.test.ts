import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const mocks = vi.hoisted(() => {
  const PrismWebRuntime = { create: vi.fn() };
  return {
    compileStreaming: vi.fn(async () => ({ compiled: true })),
    createInstance: vi.fn(async () => ({ PrismWebRuntime })),
    fetch: vi.fn(async () => new Response()),
    PrismWebRuntime,
    runtimeOptions: undefined as { loadRenderer: () => Promise<unknown> } | undefined,
  };
});

vi.mock('@typie/prism-ui', () => ({
  createPrismRuntime: (options: typeof mocks.runtimeOptions) => {
    mocks.runtimeOptions = options;
    return {};
  },
}));
vi.mock('@typie/prism-ui-web/browser', () => ({ createInstance: mocks.createInstance }));
vi.mock('@typie/prism-ui-web/browser/wasm?url', () => ({ default: '/prism-ui.wasm' }));

describe('Typie Prism renderer loader', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});

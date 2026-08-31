import { createPrismRuntime } from '@typie/prism-ui';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const observer = class {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
};

const mediaQuery = {
  addEventListener: vi.fn(),
  matches: false,
  removeEventListener: vi.fn(),
};

beforeEach(() => {
  vi.stubGlobal('IntersectionObserver', observer);
  vi.stubGlobal('ResizeObserver', observer);
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => mediaQuery),
  );
  vi.stubGlobal(
    'requestAnimationFrame',
    vi.fn(() => 1),
  );
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  Object.defineProperty(navigator, 'gpu', { configurable: true, value: {} });
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(
    () =>
      ({
        clearRect: vi.fn(),
        fillRect: vi.fn(),
        fillStyle: '',
        getImageData: vi.fn(() => ({ data: new Uint8ClampedArray([255, 255, 255, 255]) })),
      }) as never,
  );
});

afterEach(() => {
  Reflect.deleteProperty(navigator, 'gpu');
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

const createRuntimeFixture = () => {
  const renderer = {
    frameUniformByteLength: 656,
    free: vi.fn(),
    render: vi.fn(),
    renderComputed: vi.fn(),
    setHdrEnabled: vi.fn(),
    whenSubmittedWorkDone: vi.fn(() => Promise.resolve()),
  };
  const rendererRuntime = {
    createRenderer: vi.fn(() => renderer),
    free: vi.fn(),
  };
  const runtime = createPrismRuntime({
    loadRenderer: async () => ({ PrismWebRuntime: { create: vi.fn(async () => rendererRuntime) } }) as never,
  });
  return { renderer, rendererRuntime, runtime };
};

describe('Prism runtime lifecycle', () => {
  it('destroys every mount and frees the shared WASM runtime once', async () => {
    const { renderer, rendererRuntime, runtime } = createRuntimeFixture();
    const firstHost = document.createElement('div');
    const secondHost = document.createElement('div');
    document.body.append(firstHost, secondHost);
    runtime.mountObject(firstHost, { target: 'prism' });
    runtime.mountObject(secondHost, { target: 'icon' });
    await vi.waitFor(() => expect(rendererRuntime.createRenderer).toHaveBeenCalledOnce());

    runtime.destroy();
    runtime.destroy();

    await vi.waitFor(() => expect(rendererRuntime.free).toHaveBeenCalledOnce());
    expect(renderer.free).toHaveBeenCalledOnce();
    expect(firstHost.childElementCount).toBe(0);
    expect(secondHost.childElementCount).toBe(0);
    expect(() => runtime.mountObject(firstHost, { target: 'icon' })).toThrow('destroyed');
  });

  it('continues cleanup when one mount fails to destroy', async () => {
    const { rendererRuntime, runtime } = createRuntimeFixture();
    const firstHost = document.createElement('div');
    const secondHost = document.createElement('div');
    document.body.append(firstHost, secondHost);
    const firstMount = runtime.mountObject(firstHost, { target: 'prism' });
    runtime.mountObject(secondHost, { target: 'icon' });
    await vi.waitFor(() => expect(rendererRuntime.createRenderer).toHaveBeenCalledOnce());
    const error = new Error('mount cleanup failed');
    const destroyFirstMount = firstMount.destroy.bind(firstMount);
    firstMount.destroy = vi.fn(() => {
      throw error;
    });
    vi.spyOn(console, 'error').mockImplementation(() => false);

    expect(() => runtime.destroy()).not.toThrow();

    await vi.waitFor(() => expect(rendererRuntime.free).toHaveBeenCalledOnce());
    expect(secondHost.childElementCount).toBe(0);
    expect(console.error).toHaveBeenCalledWith('Failed to destroy a Prism mount.', error);
    destroyFirstMount();
  });
});

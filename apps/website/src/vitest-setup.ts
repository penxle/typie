import { vi } from 'vitest';

if (!window.matchMedia) {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => null,
      removeEventListener: () => null,
      addListener: () => null,
      removeListener: () => null,
      dispatchEvent: () => false,
    }),
  });
}

vi.mock('$lib/editor-ffi/surface-probe', () => ({
  probeAttach: vi.fn(),
  probeDetach: vi.fn(),
  probeEvent: vi.fn(),
  probeRendered: vi.fn(),
}));

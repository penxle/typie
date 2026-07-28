import { vi } from 'vitest';

vi.mock('$lib/editor-ffi/surface-probe', () => ({
  probeAttach: vi.fn(),
  probeDetach: vi.fn(),
  probeEvent: vi.fn(),
  probeRendered: vi.fn(),
}));

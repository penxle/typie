import { describe, expect, it, vi } from 'vitest';
import { checkBootstrapAssertion } from './bootstrap';

describe('checkBootstrapAssertion', () => {
  it('turns a failed fetch into a recoverable page error', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockRejectedValue(new TypeError('Failed to fetch'));

    await expect(checkBootstrapAssertion(fetch)).rejects.toMatchObject({ status: 503, body: { code: 'network_error' } });
  });

  it.each([502, 503, 504])('rejects HTTP %s without a maintenance response as unavailable', async (status) => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(new Response(null, { status }));

    await expect(checkBootstrapAssertion(fetch)).rejects.toMatchObject({ status, body: { code: 'service_unavailable' } });
  });

  it('preserves an explicit maintenance response when retrying', async () => {
    const body = {
      code: 'maintenance',
      message: '점검 중이에요.',
      maintenance: { title: '서버 점검 중', message: '점검 중이에요.', until: null },
    };
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(Response.json(body, { status: 503 }));

    await expect(checkBootstrapAssertion(fetch)).rejects.toMatchObject({ status: 503, body });
  });

  it('allows loading to continue for a normal bootstrap response', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(Response.json(null));

    await expect(checkBootstrapAssertion(fetch)).resolves.toBeUndefined();
  });
});

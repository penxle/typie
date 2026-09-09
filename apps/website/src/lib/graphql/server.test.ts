import { createServer } from 'node:net';
import { AggregatedError } from '@mearie/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { POST } from '../../routes/graphql/+server';
import { loadQuery } from './server';
import type { Artifact } from '@mearie/svelte';

const privateEnv = vi.hoisted(() => ({ PRIVATE_API_URL: 'http://api.test' }));
vi.mock('$app/environment', () => ({ browser: false, dev: true }));
vi.mock('$env/dynamic/private', () => ({ env: privateEnv }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('./client', () => ({ mearieClient: null, scalars: {} }));

const query = {
  kind: 'query',
  name: 'PageQuery',
  body: 'query PageQuery { __typename }',
  selections: [{ kind: 'Field', name: '__typename', type: 'String' }],
} satisfies Artifact<'query'>;

const url = new URL('https://typie.test/document');

const cookies = {
  get: (name: string) => (name === 'typie-did' ? 'test-device' : undefined),
  delete: vi.fn(),
};

const proxyFetch: typeof fetch = async (input, init) => {
  expect(input).toBe('/graphql');
  return POST({
    request: new Request(new URL('/graphql', url), init),
    cookies,
    getClientAddress: () => '127.0.0.1',
  } as unknown as Parameters<typeof POST>[0]);
};

afterEach(() => {
  vi.unstubAllGlobals();
  cookies.delete.mockClear();
  privateEnv.PRIVATE_API_URL = 'http://api.test';
});

describe('loadQuery errors', () => {
  it('turns a failed fetch into a recoverable page error', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockRejectedValue(new TypeError('Failed to fetch'));

    await expect(loadQuery({ fetch, url }, query)).rejects.toMatchObject({ status: 503, body: { code: 'network_error' } });
  });

  it.each([502, 503, 504])('treats HTTP %s as unavailable without inventing a maintenance response', async (status) => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(new Response(null, { status }));

    await expect(loadQuery({ fetch, url }, query)).rejects.toMatchObject({ status, body: { code: 'service_unavailable' } });
  });

  it('preserves the authentication redirect', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(new Response(null, { status: 401 }));

    await expect(loadQuery({ fetch, url }, query)).rejects.toMatchObject({ status: 302, location: url.href });
  });

  it('does not classify invalid JSON as a connection failure', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(new Response('invalid JSON'));

    await expect(loadQuery({ fetch, url }, query)).rejects.toBeInstanceOf(AggregatedError);
  });
});

describe('GraphQL proxy errors', () => {
  it('returns 502 when the API refuses connections and exposes a recoverable page error', async () => {
    const server = createServer();
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const address = server.address();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    if (!address || typeof address === 'string') throw new Error('Missing server address');
    privateEnv.PRIVATE_API_URL = `http://127.0.0.1:${address.port}`;

    const response = await proxyFetch('/graphql', { method: 'POST', body: '{}' });
    expect(response.status).toBe(502);
    await expect(loadQuery({ fetch: proxyFetch, url }, query)).rejects.toMatchObject({
      status: 502,
      body: { code: 'service_unavailable' },
    });
  });

  it('returns 502 when the connection fails while reading the API response body', async () => {
    const body = new ReadableStream({ start: (controller) => controller.error(new TypeError('terminated')) });
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(body)));

    await expect(proxyFetch('/graphql', { method: 'POST', body: '{}' })).resolves.toMatchObject({ status: 502 });
  });

  it.each([401, 500])('preserves an API HTTP %s response and authentication handling', async (status) => {
    const body = { errors: [{ message: 'API error' }] };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(body, { status })));

    const response = await proxyFetch('/graphql', { method: 'POST', body: '{}' });
    expect(response.status).toBe(status);
    await expect(response.json()).resolves.toEqual(body);
    expect(cookies.delete.mock.calls).toEqual(status === 401 ? [['typie-at', { path: '/' }]] : []);
  });

  it('does not hide unexpected proxy errors as API connection failures', async () => {
    const failure = new Error('Unexpected proxy failure');
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(failure));

    await expect(proxyFetch('/graphql', { method: 'POST', body: '{}' })).rejects.toBe(failure);
  });
});

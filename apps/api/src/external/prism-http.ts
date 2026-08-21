import { Readable } from 'node:stream';
import got from 'got';
import type { Method } from 'got';
import type { PrismHttp } from './prism-core.ts';

const TIMEOUT_MS = 10_000;
const TOTAL_TIMEOUT_MS = 30_000;
const RETRY_LIMIT = 2;
const BACKOFF_BASE_MS = 300;

export type PrismHttpOptions = {
  baseUrl: string;
  token: string;
  timeout?: number;
  totalTimeout?: number;
  retryLimit?: number;
  onRetry?: (info: { method: string; path: string; attempt: number; error: Error }) => void;
};

export const createPrismHttp = (options: PrismHttpOptions): PrismHttp => {
  const client = got.extend({
    http2: true,
    throwHttpErrors: false,
    headers: { authorization: `Bearer ${options.token}` },
  });

  return {
    request: async (path, init) => {
      const method = (init?.method ?? 'GET') as Method;
      const url = `${options.baseUrl}${path}`;

      if (init?.stream) {
        const stream = client.stream(url, { method, headers: init.headers, json: init.body, signal: init.signal, retry: { limit: 0 } });
        const response = await new Promise<{ statusCode: number }>((resolve, reject) => {
          stream.once('response', resolve);
          stream.once('error', reject);
        });
        const body = Readable.toWeb(stream) as ReadableStream<Uint8Array>;
        return { status: response.statusCode, json: () => new Response(body).json(), body };
      }

      const perAttempt = options.timeout ?? TIMEOUT_MS;
      const totalTimeout = options.totalTimeout ?? TOTAL_TIMEOUT_MS;
      const startedAt = Date.now();
      const remaining = () => totalTimeout - (Date.now() - startedAt);

      const res = await client(url, {
        method,
        headers: init?.headers,
        json: init?.body,
        signal: init?.signal,
        timeout: { request: perAttempt },
        retry: {
          limit: options.retryLimit ?? RETRY_LIMIT,
          methods: ['GET', 'POST', 'PUT'],
          statusCodes: [],
          calculateDelay: ({ attemptCount, computedValue }) => {
            if (computedValue === 0) return 0;
            const left = remaining();
            return left <= 0 ? 0 : Math.min(BACKOFF_BASE_MS * 2 ** (attemptCount - 1), left);
          },
        },
        hooks: {
          beforeRetry: [
            (error, retryCount) => {
              error.options.timeout = { request: Math.max(1, Math.min(perAttempt, remaining())) };
              options.onRetry?.({ method, path, attempt: retryCount, error });
            },
          ],
        },
      });

      return { status: res.statusCode, json: async () => JSON.parse(res.body), body: null };
    },
  };
};

import { Readable } from 'node:stream';
import got from 'got';
import { Agent as Http2Agent } from 'http2-wrapper';
import type { Method } from 'got';
import type { PrismHttp } from './prism-core.ts';

const TIMEOUT_MS = 10_000;
const TOTAL_TIMEOUT_MS = 30_000;
const STREAM_OPEN_TIMEOUT_MS = 15_000;
const SESSION_IDLE_TIMEOUT_MS = 30_000;
const RETRY_LIMIT = 2;
const BACKOFF_BASE_MS = 300;

export type PrismHttpOptions = {
  baseUrl: string;
  token: string;
  timeout?: number;
  totalTimeout?: number;
  streamOpenTimeout?: number;
  http2Agent?: Http2Agent;
  retryLimit?: number;
  onRetry?: (info: { method: string; path: string; attempt: number; error: Error }) => void;
};

export const createPrismHttp = (options: PrismHttpOptions): PrismHttp => {
  const http2Agent = options.http2Agent ?? new Http2Agent({ timeout: SESSION_IDLE_TIMEOUT_MS });
  const client = got.extend({
    http2: true,
    throwHttpErrors: false,
    agent: { http2: http2Agent },
    headers: { authorization: `Bearer ${options.token}` },
  });

  return {
    request: async (path, init) => {
      const method = (init?.method ?? 'GET') as Method;
      const url = `${options.baseUrl}${path}`;

      if (init?.stream) {
        const stream = client.stream(url, { method, headers: init.headers, json: init.body, signal: init.signal, retry: { limit: 0 } });
        // 열기(응답 헤더 대기)에만 상한을 건다 — 본문은 SSE라 무기한이 계약이지만, 반쯤 죽은 연결 위의
        // 열기 대기는 response도 error도 영영 오지 않아 소비자(펌프)의 유휴 감시까지 비켜간다.
        const response = await new Promise<{ statusCode: number }>((resolve, reject) => {
          const timer = setTimeout(() => {
            // 열기가 응답 없이 시간을 넘긴 세션은 꼬였을 가능성이 높다 — 다음 시도가 재사용하지 않게 전부 파기한다.
            http2Agent.destroy();
            stream.destroy();
            reject(new Error(`stream open timed out: ${method} ${path}`));
          }, options.streamOpenTimeout ?? STREAM_OPEN_TIMEOUT_MS);
          stream.once('response', (res: { statusCode: number }) => {
            clearTimeout(timer);
            resolve(res);
          });
          stream.once('error', (error: Error) => {
            clearTimeout(timer);
            reject(error);
          });
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
              http2Agent.destroy();
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

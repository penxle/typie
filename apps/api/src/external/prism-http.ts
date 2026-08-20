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

// 전송은 got + HTTP/2(오너 결정 2026-08-21). 열린 SSE가 같은 origin의 단발 요청을 굶기지 않아야 하는데, got의
// h2 경로(http2-wrapper — got의 자체 의존성, 별도 설치 없음)는 한 세션에 스트림을 멀티플렉스하고 h1 폴백도 다중
// 소켓이라 기아가 없다. Node 내장 fetch에 npm undici Agent를 꽂던 이전 배선은 두 undici(내장·npm)의 핸들러
// 프로토콜이 버전에 따라 어긋나 프로덕션에서 죽었다(2026-08-20 `invalid onError method`) — 같은 함정이 없는 Node
// 코어 http/http2 스택을 쓴다. api에서 got을 쓰는 곳은 여기뿐이라 http2-wrapper의 전역 에이전트가 곧 전용 풀이다.
// POST 재시도는 prism의 key 멱등(같은 key = 기존 run 반환)과 cancel의 no-op 멱등에 기댄다.
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
        // 스트림에는 시도 상한도 전체 예산도 없다(SSE는 살아 있는 동안 유지). status는 응답 헤더 도착 시점에 확정한다.
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
          methods: ['GET', 'POST'],
          statusCodes: [], // 상태 기반 재시도는 하지 않는다 — HTTP 오류의 해석은 prism-core의 몫이다
          // 전체 예산: 남은 예산이 없으면 재시도를 멈추고(0), 있으면 ky 시절과 같은 300ms 지수 백오프를 예산 안에서 쓴다.
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

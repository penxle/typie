import ky from 'ky';
import { Agent } from 'undici';
import type { PrismHttp } from './prism-core.ts';

const TIMEOUT_MS = 10_000;
const TOTAL_TIMEOUT_MS = 30_000;
const RETRY_LIMIT = 2;

export type PrismHttpOptions = {
  baseUrl: string;
  token: string;
  timeout?: number;
  totalTimeout?: number;
  retryLimit?: number;
  onRetry?: (info: { method: string; path: string; attempt: number; error: Error }) => void;
};

// POST 재시도는 prism의 key 멱등(같은 key = 기존 run 반환)과 cancel의 no-op 멱등에 기댄다.
// 전용 dispatcher가 필수인 이유: Node 글로벌 dispatcher는 origin당 연결 1개라 열린 SSE가
// 이후 모든 요청을 기아시킨다(실측). allowH2로 SSE와 단발 요청이 한 연결에 멀티플렉스된다.
export const createPrismHttp = (options: PrismHttpOptions): PrismHttp => {
  const dispatcher = new Agent({ allowH2: true });

  return {
    request: async (path, init) => {
      const method = init?.method ?? 'GET';
      const res = await ky(`${options.baseUrl}${path}`, {
        method,
        headers: { authorization: `Bearer ${options.token}`, ...init?.headers },
        json: init?.body,
        signal: init?.signal,
        timeout: init?.stream ? false : (options.timeout ?? TIMEOUT_MS),
        totalTimeout: init?.stream ? undefined : (options.totalTimeout ?? TOTAL_TIMEOUT_MS),
        throwHttpErrors: false,
        retry: { limit: options.retryLimit ?? RETRY_LIMIT, methods: ['get', 'post'], retryOnTimeout: true },
        hooks: { beforeRetry: [({ error, retryCount }) => options.onRetry?.({ method, path, attempt: retryCount, error })] },
        dispatcher,
      } as Parameters<typeof ky>[1]);

      return { status: res.status, json: () => res.json(), body: res.body };
    },
  };
};

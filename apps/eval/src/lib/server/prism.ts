import type { PrismRun, PrismSessionView } from '../feedback/types.ts';

type PrismEnv = { PRISM_API_ORIGIN: string; PRISM_API_TOKEN: string };

export class PrismApiError extends Error {
  readonly code: string;
  readonly status: number;

  constructor(code: string, status: number) {
    super(`prism-api ${status}: ${code}`);
    this.name = 'PrismApiError';
    this.code = code;
    this.status = status;
  }
}

export const newPrismSessionId = (): string => `ev-${crypto.randomUUID()}`;

const call = async (env: PrismEnv, path: string, init?: RequestInit): Promise<Response> => {
  const headers: Record<string, string> = { authorization: `Bearer ${env.PRISM_API_TOKEN}` };
  if (init?.body) headers['content-type'] = 'application/json';
  const res = await fetch(`${env.PRISM_API_ORIGIN}${path}`, { ...init, headers: { ...headers, ...init?.headers } });
  if (!res.ok) {
    const code = await res
      .json()
      .then((b) => (b as { error?: string }).error ?? 'internal')
      .catch(() => 'internal');
    throw new PrismApiError(code, res.status);
  }
  return res;
};

export const startWorkflow = async (
  env: PrismEnv,
  opts: { sessionId: string; input: { manuscriptPath: string }; files: { path: string; content: string }[] },
): Promise<void> => {
  await call(env, '/workflows', {
    method: 'POST',
    body: JSON.stringify({ sessionId: opts.sessionId, app: 'feedback', workflow: 'main', input: opts.input, files: opts.files }),
  });
};

// 와이어의 result·usage는 JSON "문자열"이다 — prism은 종결 시 JSON.stringify로 저장하고(workflow.ts:383·105)
// 뷰는 text 컬럼을 파싱 없이 그대로 싣는다(do.ts RunRow). 객체 복원은 이 경계의 책임이다.
type WireRun = Omit<PrismRun, 'result' | 'usage'> & { result: string | null; usage: string | null };

export const getSession = async (env: PrismEnv, id: string): Promise<PrismSessionView> => {
  const res = await call(env, `/sessions/${id}`);
  const raw = (await res.json()) as { session: Record<string, unknown>; runs: WireRun[] };
  return {
    session: raw.session,
    runs: raw.runs.map((run) => ({
      ...run,
      result: run.result === null ? null : (JSON.parse(run.result) as PrismRun['result']),
      usage: run.usage === null ? null : (JSON.parse(run.usage) as PrismRun['usage']),
    })),
  };
};

export const cancelRun = async (env: PrismEnv, id: string): Promise<void> => {
  await call(env, `/sessions/${id}/cancel`, { method: 'POST', body: JSON.stringify({ runSeq: 1 }) });
};

export const openEvents = (env: PrismEnv, id: string, lastEventId: number): Promise<Response> =>
  call(env, `/sessions/${id}/events?lastEventId=${lastEventId}`);

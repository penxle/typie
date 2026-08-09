import type { AskAnswer } from '../feedback/live.ts';
import type { TierName, TierOverrides } from '../feedback/tiers.ts';
import type { PrismWorkflow, PrismWorkflowView } from '../feedback/types.ts';

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

export const newPrismWorkflowId = (): string => `ev-${crypto.randomUUID()}`;

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
  opts: {
    workflowId: string;
    workflow: TierName;
    input: { manuscriptPath: string; overrides?: TierOverrides };
    files: { path: string; content: string }[];
  },
): Promise<void> => {
  await call(env, '/workflows', {
    method: 'POST',
    body: JSON.stringify({ workflowId: opts.workflowId, app: 'feedback', workflow: opts.workflow, input: opts.input, files: opts.files }),
  });
};

// 와이어의 result는 JSON "문자열"이다(prism core/workflow.ts의 driveWorkflow가 JSON.stringify해 종결한다).
// usage는 settled 판별자를 단 객체로 온다 — eval·prism 동시 배포가 전제라 과도기 분기가 없다.
// 객체 복원 책임은 result에만 남는다.
type WireWorkflow = Omit<PrismWorkflow, 'result'> & { result: string | null };

export const getWorkflow = async (env: PrismEnv, id: string): Promise<PrismWorkflowView> => {
  const res = await call(env, `/workflows/${id}`);
  const raw = (await res.json()) as { workflow: WireWorkflow };
  const wf = raw.workflow;
  // 명시 사영 — 와이어 행(14열)에는 caller·input 등 화면이 몰라야 할 열이 실려 온다. 전개로 옮기면 타입 밖
  // 필드가 런타임 객체에 남아 이후 소비처로 조용히 새므로, 이 경계가 통과시키는 것을 전부 열거한다.
  return {
    workflow: {
      status: wf.status,
      error: wf.error,
      startedAt: wf.startedAt,
      finishedAt: wf.finishedAt,
      result: wf.result === null ? null : (JSON.parse(wf.result) as PrismWorkflow['result']),
      usage: wf.usage,
    },
  };
};

export const cancelWorkflow = async (env: PrismEnv, id: string): Promise<void> => {
  await call(env, `/workflows/${id}/cancel`, { method: 'POST' });
};

export const openEvents = (env: PrismEnv, id: string, lastEventId: number): Promise<Response> =>
  call(env, `/workflows/${id}/events?lastEventId=${lastEventId}`);

// 이벤트 로그의 정지 사진 — SSE 재생과 같은 행을 JSON으로 한 번에 받는다(실행 중 세션의 첫 화면 시드용).
export type PrismLogEvent = { seq: number; kind: string; data: Record<string, unknown>; createdAt: number };

export const fetchEventLog = async (env: PrismEnv, id: string): Promise<PrismLogEvent[]> => {
  const res = await call(env, `/workflows/${id}/log`);
  return ((await res.json()) as { events: PrismLogEvent[] }).events;
};

// 두 세그먼트는 폼에서 온 클라이언트 입력이다 — 날것으로 보간하면 URL 파서의 dot-segment 정규화로 ../ 순회가
// 성립해, 호출부의 워크플로 소속 가드를 통과한 뒤 남의 에이전트 해소 경로로 요청이 나간다.
export const resolveAskUser = async (env: PrismEnv, agentId: string, toolCallId: string, answers: AskAnswer[]): Promise<void> => {
  const path = `/agents/${encodeURIComponent(agentId)}/tools/${encodeURIComponent(toolCallId)}/result`;
  await call(env, path, { method: 'POST', body: JSON.stringify({ result: { answers } }) });
};

export const getWorkflowInvocations = async (
  env: PrismEnv,
  workflowId: string,
): Promise<{ agentId: string; agentName: string; status: string }[]> => {
  const res = await call(env, `/workflows/${workflowId}`);
  const raw = (await res.json()) as { invocations: { agentId: string; agentName: string; status: string }[] };
  return raw.invocations.map((i) => ({ agentId: i.agentId, agentName: i.agentName, status: i.status }));
};

export const getAgentPendingTool = async (env: PrismEnv, agentId: string): Promise<{ toolCallId: string; tool: string } | null> => {
  const res = await call(env, `/agents/${agentId}`);
  const raw = (await res.json()) as { pending: { toolCallId: string; tool: string } | null };
  return raw.pending === null ? null : { toolCallId: raw.pending.toolCallId, tool: raw.pending.tool };
};

// 목록 배지용 2홉 — 파이프라인이 직렬이라 running invocation은 통상 1개다(베타 규모 전제로 순차 조회).
export const hasPendingQuestion = async (env: PrismEnv, workflowId: string): Promise<boolean> => {
  const invocations = await getWorkflowInvocations(env, workflowId);
  for (const invocation of invocations) {
    if (invocation.status !== 'running') continue;
    const pending = await getAgentPendingTool(env, invocation.agentId);
    if (pending?.tool === 'ask-user') return true;
  }
  return false;
};

// ask-user 성공 해소만 원장에 실리므로(도구 오류 문면 커밋은 data가 없다) 필터가 곧 해소 이력이다.
export const getAgentAskCalls = async (env: PrismEnv, agentId: string): Promise<AskAnswer[][]> => {
  const res = await call(env, `/agents/${agentId}/calls`);
  const raw = (await res.json()) as { calls: { tool: string; data: unknown }[] };
  return raw.calls
    .filter(
      (c) =>
        c.tool === 'ask-user' && c.data !== null && typeof c.data === 'object' && Array.isArray((c.data as { answers?: unknown }).answers),
    )
    .map((c) => (c.data as { answers: AskAnswer[] }).answers);
};

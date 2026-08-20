import type { AskAnswer } from '../feedback/live.ts';
import type { PriceTable } from '../feedback/pricing.ts';
import type { AppCatalog, TierName, TierOverrides } from '../feedback/tiers.ts';
import type { ManuscriptMeta, PreviousInput, PrismWorkflow, PrismWorkflowView } from '../feedback/types.ts';

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

// 앱 카탈로그 — 시작 폼 옵션·제출 검증·스냅샷 조립의 재료다. 무캐시(오너 결정 2026-08-13): 시작은 드문
// 이벤트라 매번 걷고, 실패 시 폴백 없이 시작만 막는다(열람 화면은 이 표면과 무관하다).
export const fetchCatalog = async (env: PrismEnv): Promise<AppCatalog> => {
  const res = await call(env, '/apps/feedback/catalog');
  return (await res.json()) as AppCatalog;
};

// 모델 단가표 — 정본은 prism이고(prism src/pricing.ts) 이 라우트는 그 상수를 그대로 싣는다. 원가 표시는
// 부가 정보라 수신 실패가 화면을 막지 않는다: null을 돌려 전 비용이 미상(—)으로 강등된다(pricing.ts의 표
// 부재 처리). 무캐시 — 열람마다 걷는다(베타 규모라 호출 비용이 사소하고, 스테일 캐시 기제를 두지 않는다).
// 무음이면 강등 반복을 화면의 —로만 알 수 있으므로 로그는 남긴다(wrangler tail로 관측).
export const fetchPriceTable = async (env: PrismEnv): Promise<PriceTable | null> => {
  try {
    const res = await call(env, '/pricing');
    return (await res.json()) as PriceTable;
  } catch (err) {
    console.error('price table fetch failed', err);
    return null;
  }
};

type SeedFile = { path: string; content: string };
type SeedDigest = { path: string; bytes: number; sha256: string };

// 시드 묶음 1회의 와이어 예산 — prism API 워커는 PUT /seeds 바디를 표면 RPC 1회(직렬화 32MiB 한도)로
// 나른다. 와이어는 V8 직렬화라 비Latin-1 문자가 하나라도 섞인 문자열이 UTF-16 유닛당 2바이트로 실리는
// 것이 최악이다(prism core/wire.ts 실측, 스펙 2026-08-20-r2-offload §2) — 유닛×2 합산이 이 예산 아래면
// 내용과 무관하게 안전하다. 24MiB는 32MiB 대비 봉투·경로 오버헤드를 덮는 여유폭이다.
const SEED_BATCH_BUDGET_BYTES = 24 * 1024 * 1024;

const seedWireCost = (f: SeedFile): number => (f.path.length + f.content.length) * 2 + 64;

// 총합 무제한 시딩 — 파일을 예산 이하 묶음으로 선적재하고 start에는 매니페스트(digest)만 싣는다(prism 스펙
// §4: inline files는 start RPC 1회에 전부 실려 32MiB 와이어가 총합 상한이 된다 — 원고 수십~수백 개 시딩 대응).
// 소형/대형 이중 경로를 두지 않고 항상 적재한다 — 시작은 드문 이벤트라 왕복 1회 추가가 사소하고, 큰 시딩에서만
// 밟히는 별도 경로를 남기지 않는다. budget 인자는 테스트 전용(분할 경계를 작은 값으로 재현).
export const stageSeeds = async (
  env: PrismEnv,
  workflowId: string,
  files: SeedFile[],
  budget: number = SEED_BATCH_BUDGET_BYTES,
): Promise<SeedDigest[]> => {
  const staged: SeedDigest[] = [];
  let batch: SeedFile[] = [];
  let cost = 0;
  const flush = async (): Promise<void> => {
    if (batch.length === 0) return;
    const res = await call(env, `/workflows/${workflowId}/seeds`, { method: 'PUT', body: JSON.stringify({ files: batch }) });
    staged.push(...((await res.json()) as { staged: SeedDigest[] }).staged);
    batch = [];
    cost = 0;
  };
  for (const file of files) {
    const c = seedWireCost(file);
    // 예산을 넘기면 앞 묶음을 먼저 보낸다 — 단일 파일이 예산을 넘는 경우는 혼자 한 묶음이 된다(스펙 전제:
    // 단일 파일 ≤ 32MiB 와이어).
    if (batch.length > 0 && cost + c > budget) await flush();
    batch.push(file);
    cost += c;
  }
  await flush();
  return staged;
};

export const startWorkflow = async (
  env: PrismEnv,
  opts: {
    workflowId: string;
    workflow: TierName;
    // previous가 실리면 재리뷰다 — 워크플로 이름은 그대로고 이 키의 유무가 모드를 가른다.
    input: { manuscriptPath: string; meta: ManuscriptMeta; overrides?: TierOverrides; previous?: PreviousInput };
    files: SeedFile[];
  },
): Promise<void> => {
  // 적재 → 매니페스트 start. 도중 실패하면 그대로 던진다 — 호출부(reviews.ts)가 회차를 failed로 귀속하고,
  // 재시도는 새 workflowId로 시작하므로 남은 적재분(고아 staging)은 무해하다(prism 스펙 §4-4).
  const staged = await stageSeeds(env, opts.workflowId, opts.files);
  await call(env, '/workflows', {
    method: 'POST',
    body: JSON.stringify({ workflowId: opts.workflowId, app: 'feedback', workflow: opts.workflow, input: opts.input, staged }),
  });
};

export const fetchWorkflowFile = async (env: PrismEnv, id: string, path: string): Promise<string | null> => {
  const res = await call(env, `/workflows/${id}/files/${path}`);
  return ((await res.json()) as { content: string | null }).content;
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

// 실패 종결 워크플로의 부분 재개 — 완료 스텝은 재생되고 실패 invocation만 새 run으로 이어진다(prism core/do.ts
// retryWorkflow). 전제 위반은 retry-rejected·retry-unsettled(409)로, 워크플로 부재는 404로 돌아온다.
export const retryWorkflow = async (env: PrismEnv, id: string): Promise<void> => {
  await call(env, `/workflows/${id}/retry`, { method: 'POST' });
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

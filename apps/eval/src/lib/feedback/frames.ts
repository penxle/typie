// prism 이벤트 봉투({seq,kind,occurredAt,loggedAt,context,data} — prism docs/events.md §3)의 eval 사영.
// 이벤트는 전문을 싣는다(도구 input·output 원문, 턴 raw — MB 단위 가능, §11) — 리듀서(live.ts)가 읽는 필드만
// 남기고 소화하지 않는 kind는 떨군다. 릴레이·첫 화면 시드·종결 사영 저장이 같은 사영을 거치고, 저장 경계에서는
// D1 행 한도(2MB) 때문에 필수다. 봉투의 부분집합이며 파생 필드는 input.chars(write content 길이) 하나다.
// 구세대 행(context: null — §9)은 떨군다: 구 어휘 해석 의무가 없고, 구 세션의 과정 화면이 서지 않는 것은
// 파이프라인 컷오버의 선례와 같은 수용이다.

export type AgentRef = { id: string; name: string };

export type FrameContext = {
  step?: string;
  agent?: AgentRef;
  run?: number;
  turn?: number;
  attempt?: number;
  toolCallId?: string;
};

export type Frame = {
  seq: number;
  kind: string;
  occurredAt: number | null;
  context: FrameContext;
  data: Record<string, unknown>;
};

// 리듀서가 소화하는 kind(live.ts CONSUMED_EVENTS) + turn.started(흐르는 턴 조각의 유통기한, delta.ts).
// 화면 구독 목록(sse.ts EVENT_NAMES)도 이것이다 — 결속은 sse.test.ts.
export const PROJECTED_KINDS = [
  'workflow.started',
  'step.started',
  'step.completed',
  'turn.started',
  'turn.completed',
  'tool.requested',
  'tool.executed',
  'tool.resolved',
  'workflow.completed',
  'workflow.failed',
  'workflow.canceled',
  'workflow.retried',
] as const;

const PROJECTED = new Set<string>(PROJECTED_KINDS);

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);
const str = (value: unknown): string | null => (typeof value === 'string' ? value : null);
const int = (value: unknown): number | null => (typeof value === 'number' && Number.isFinite(value) ? value : null);

const contextOf = (raw: Record<string, unknown>): FrameContext => {
  const context: FrameContext = {};
  const step = str(raw.step);
  if (step !== null) context.step = step;
  if (isRecord(raw.agent)) {
    const id = str(raw.agent.id);
    if (id !== null) context.agent = { id, name: str(raw.agent.name) ?? '' };
  }
  for (const key of ['run', 'turn', 'attempt'] as const) {
    const value = int(raw[key]);
    if (value !== null) context[key] = value;
  }
  const toolCallId = str(raw.toolCallId);
  if (toolCallId !== null) context.toolCallId = toolCallId;
  return context;
};

const usageOf = (raw: unknown): Record<string, unknown> | null => {
  if (!isRecord(raw)) return null;
  const { inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens, thinkingTokens } = raw;
  return { inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens, thinkingTokens: thinkingTokens ?? null };
};

const inputOf = (raw: unknown): Record<string, unknown> => {
  if (!isRecord(raw)) return {};
  const input: Record<string, unknown> = {};
  const path = str(raw.path);
  if (path !== null) input.path = path;
  const query = str(raw.query);
  if (query !== null) input.query = query;
  const content = str(raw.content);
  if (content !== null) input.chars = content.length;
  return input;
};

const dataOf = (kind: string, data: Record<string, unknown>): Record<string, unknown> => {
  switch (kind) {
    case 'turn.completed': {
      return { text: str(data.text), usage: usageOf(data.usage) };
    }
    case 'tool.requested': {
      // data = 호스트로 나가는 요청 페이로드(ask-user의 {questions}, §6.3) — 질문 카드의 실물
      return data.tool === 'ask-user' ? { tool: data.tool, data: data.data ?? null } : { tool: data.tool };
    }
    case 'tool.resolved': {
      // data = 제출된 결과 원본(ask-user의 {answers}) — 답변 문면의 유일한 원천(원장 표면은 없다, §10)
      return data.tool === 'ask-user' ? { tool: data.tool, ok: data.ok, data: data.data ?? null } : { tool: data.tool, ok: data.ok };
    }
    case 'tool.executed': {
      return { tool: data.tool, ok: data.ok, input: inputOf(data.input) };
    }
    default: {
      return {};
    }
  }
};

export const projectFrame = (raw: unknown): Frame | null => {
  if (!isRecord(raw)) return null;
  const seq = int(raw.seq);
  const kind = str(raw.kind);
  if (seq === null || kind === null || !isRecord(raw.context) || !PROJECTED.has(kind)) return null;
  return {
    seq,
    kind,
    occurredAt: int(raw.occurredAt),
    context: contextOf(raw.context),
    data: dataOf(kind, isRecord(raw.data) ? raw.data : {}),
  };
};

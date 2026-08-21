import { nestedRound, STAGES, stepStage, TERMINAL_EVENTS } from './stages.ts';
import type { FrameContext } from './frames.ts';
import type { SseEvent } from './sse.ts';
import type { StageKey } from './stages.ts';

export type StageStatus = 'pending' | 'running' | 'done';

// 활동 라인은 모델이 턴을 확정하며 남긴 발화다(turn.completed data의 text — prism docs/events.md §6.2).
// 도구 호출은 라인을 만들지 않고, 기술 요약(경로·좌표)도 화면에 싣지 않는다.
export type ActivityLine = { id: number; text: string; stage: StageKey | null; step: string | null; at: number | null };

export type StageTiming = { firstAt: number | null; lastAt: number | null };

// 턴 확정에 동봉된 usage의 에이전트별 누적 — 비용 축은 pricing.ts의 4축이라 thinkingTokens는 접지 않는다.
export type AgentUsage = { turns: number; inputTokens: number; outputTokens: number; cacheReadTokens: number; cacheWriteTokens: number };

// 검수 라운드는 발화 없이 지나갈 수 있다 — 기계적으로 끝낸 왕복에서는 모델이 입을 열지 않는다. 발화만으로 화면을
// 세우면 그런 라운드는 흔적 없이 사라지므로, 턴과 별개로 스텝 자체의 자취를 라운드마다 접어 둔다. id는 발화 라인과
// 같은 커서라(둘 다 이벤트 순번) 두 흐름을 한 줄에 세울 수 있다.
export type NestedSpan = { id: number; stage: StageKey; firstAt: number | null; lastAt: number | null };

// 도구 호출의 자취 — tool.executed(§6.3)에서 화면이 쓸 것만 추린다. read는 대상 경로로 원고/노트를 가르고,
// 실패한 호출과 좌표성 낮은 도구(list)는 남기지 않는다 — 캡슐은 활동의 요약이지 감사 로그가 아니다.
export type ToolVerb =
  'read-manuscript' | 'read-note' | 'read-scratch' | 'grep' | 'search' | 'write' | 'write-scratch' | 'edit' | 'edit-scratch';
export type ToolMark = {
  id: number;
  verb: ToolVerb;
  query: string | null;
  chars: number | null;
  stage: StageKey;
  step: string | null;
  at: number | null;
};

export type AskOption = { label: string; description?: string };
export type AskQuestion = { question: string; hint: string; multi: boolean; options: AskOption[] };
export type AskAnswer = { question: string; choice: string[] };

// 작가에게 물은 기록 — pending은 파이프라인 직렬성 덕에 한 시점 ≤ 1이지만, 기록은 배열이다:
// 앞 단계가 묻고 답한 뒤 뒤 단계가 또 물을 수 있고 answered 카드는 각자의 자리에 남아야 한다.
export type QuestionEntry = {
  id: number;
  agentId: string;
  agentName: string;
  toolCallId: string;
  questions: AskQuestion[];
  stage: StageKey | null;
  step: string | null;
  at: number | null;
  // 대기가 끝난 시각(해소·종결의 이벤트 시각) — [at, settledAt]이 파킹 구간이고, 진행 시간 표시가 이 구간을 뺀다
  settledAt: number | null;
  status: 'pending' | 'answered' | 'closed';
  // 해소 이벤트(tool.resolved)가 실은 제출 원본 — 답을 못 받고 끝났거나 형태가 어긋나면 null
  answers: AskAnswer[] | null;
};

// 파킹 구간의 공통 형태 — 질문 대기(QuestionEntry)와 실패~재개 공백(pauses)이 같은 축으로 감산된다.
export type PauseSpan = { at: number | null; settledAt: number | null };

// 파킹은 리뷰의 진행이 아니다 — 주어진 창과 겹치는 파킹 구간의 합을 잰다(진행 시간 표시의 감산항).
// 미해소 구간은 now까지 진행 중으로 치고, at을 잃은 엔트리는 잴 수 없어 건너뛴다. 창 귀속에 스테이지 필터가
// 없어도 되는 근거는 파이프라인 직렬성이다: 파킹 구간은 정확히 한 스테이지·라운드의 창 안에 놓인다.
export const pausedMs = (spans: PauseSpan[], now: number, window?: { from?: number | null; to?: number | null }): number => {
  let total = 0;
  for (const span of spans) {
    if (span.at === null) continue;
    const start = window?.from == null ? span.at : Math.max(span.at, window.from);
    const end = Math.min(span.settledAt ?? now, window?.to ?? now);
    if (end > start) total += end - start;
  }
  return total;
};

export type LiveState = {
  stages: Record<StageKey, StageStatus>;
  timing: Record<StageKey, StageTiming>;
  activity: ActivityLine[];
  marks: ToolMark[];
  questions: QuestionEntry[];
  nestedSpans: Record<number, NestedSpan>;
  usage: Record<string, AgentUsage>;
  currentStage: StageKey | null;
  currentStep: string | null;
  startedAt: number | null;
  // 실패~재개의 닫힌 구간들 — 진행 시간 표시가 질문 대기와 같은 축으로 감산한다. failedAt은 아직 재개가
  // 오지 않은 실패의 시각 표지다(재개 없이 끝나면 열린 구간은 세우지 않는다 — 종결 화면에 진행 시간이 없다).
  pauses: { at: number; settledAt: number }[];
  failedAt: number | null;
  terminal: boolean;
  cursor: number;
};

// 리듀서가 소화하는 kind 전부 — 분기는 이 목록에서 좁힌 이름으로만 하므로 목록 밖 이름으로 분기하면 타입이 막는다.
// 사영(frames.ts PROJECTED_KINDS)·구독(sse.ts EVENT_NAMES)이 이 목록을 덮어야 프레임이 리듀서에 닿는다 — 결속은 sse.test.ts.
export const CONSUMED_EVENTS = [
  'workflow.started',
  'step.started',
  'step.completed',
  'turn.completed',
  'tool.requested',
  'tool.executed',
  'tool.resolved',
  'workflow.completed',
  'workflow.failed',
  'workflow.canceled',
  'workflow.retried',
] as const;

type ConsumedEvent = (typeof CONSUMED_EVENTS)[number];

const consumedKind = (name: string): ConsumedEvent | null => CONSUMED_EVENTS.find((kind) => kind === name) ?? null;

const ACTIVITY_LIMIT = 200;
const MARKS_LIMIT = 400;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

const str = (value: unknown): string | null => (typeof value === 'string' && value.length > 0 ? value : null);
const int = (value: unknown): number | null => (typeof value === 'number' && Number.isFinite(value) ? value : null);

type Decoded = { payload: Record<string, unknown> | null; context: FrameContext; at: number | null };

// SSE data 라인은 이벤트 본문이 아니라 {seq,kind,occurredAt,loggedAt,context,data} 봉투다(prism docs/events.md §3).
// 좌표(스텝·에이전트·도구 호출)는 context에만 있고 data에는 없다(§4) — occurredAt이 UI 시간 표시의 원 시각이다
// (재생분에 수신 시각은 오답). 릴레이·시드·저장이 같은 봉투 형태를 사영해 넘긴다(frames.ts).
const decode = (raw: string): Decoded => {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { payload: null, context: {}, at: null };
  }
  if (!isRecord(parsed)) return { payload: null, context: {}, at: null };
  return {
    payload: isRecord(parsed.data) ? parsed.data : null,
    context: isRecord(parsed.context) ? (parsed.context as FrameContext) : {},
    at: int(parsed.occurredAt),
  };
};

// 질문 페이로드 — tool.requested의 data(호스트로 나가는 요청 {questions}, §6.3)이자 get 뷰 pending.data(§7)다.
// 구조가 어긋나면 null — 화면이 깨진 질문을 세우는 것보다 안 세우는 것이 낫다. 다만 문면(question·hint·label)은
// 빈 문자열도 받는다: 스키마가 허용하는 값이라 여기서 떨어뜨리면 파킹된 run이 카드 없이 무한 대기한다.
export const parseAskQuestions = (data: unknown): AskQuestion[] | null => {
  if (!isRecord(data) || !Array.isArray(data.questions)) return null;
  const questions: AskQuestion[] = [];
  for (const q of data.questions) {
    if (!isRecord(q)) return null;
    const { question, hint } = q;
    if (typeof question !== 'string' || typeof hint !== 'string' || typeof q.multi !== 'boolean' || !Array.isArray(q.options)) return null;
    const options: AskOption[] = [];
    for (const o of q.options) {
      if (!isRecord(o) || typeof o.label !== 'string') return null;
      // description은 표시 부가물이라 빈 값은 없는 것과 같다.
      const description = str(o.description);
      options.push(description === null ? { label: o.label } : { label: o.label, description });
    }
    questions.push({ question, hint, multi: q.multi, options });
  }
  return questions.length === 0 ? null : questions;
};

const parseAskUser = (
  context: FrameContext,
  payload: Record<string, unknown> | null,
): Omit<QuestionEntry, 'id' | 'stage' | 'step' | 'at' | 'settledAt' | 'status' | 'answers'> | null => {
  if (payload?.tool !== 'ask-user' || context.agent === undefined) return null;
  const agentId = str(context.agent.id);
  const agentName = str(context.agent.name);
  const toolCallId = str(context.toolCallId);
  if (agentId === null || agentName === null || toolCallId === null) return null;
  const questions = parseAskQuestions(payload.data);
  return questions === null ? null : { agentId, agentName, toolCallId, questions };
};

// 해소 원본(tool.resolved.data = {answers}) — 형태가 어긋나면 null(대기의 끝은 그대로 굳고 문면만 비운다).
const parseAskAnswers = (data: unknown): AskAnswer[] | null => {
  if (!isRecord(data) || !Array.isArray(data.answers)) return null;
  const answers: AskAnswer[] = [];
  for (const a of data.answers) {
    if (!isRecord(a) || typeof a.question !== 'string' || !Array.isArray(a.choice)) return null;
    if (a.choice.some((c) => typeof c !== 'string')) return null;
    answers.push({ question: a.question, choice: a.choice as string[] });
  }
  return answers;
};

const append = (activity: ActivityLine[], line: ActivityLine): ActivityLine[] => [...activity, line].slice(-ACTIVITY_LIMIT);

const touch = (timing: Record<StageKey, StageTiming>, stage: StageKey, at: number | null): Record<StageKey, StageTiming> => {
  if (at === null) return timing;
  const entry = timing[stage];
  return { ...timing, [stage]: { firstAt: entry.firstAt ?? at, lastAt: at } };
};

// 라운드의 자취는 스텝이 시작하는 순간 열리고, 다음 스텝이나 종결이 닫는다. 첫 등록만은 시각이 없어도 남긴다 —
// 봉투가 깨져 시각을 잃어도 "그런 라운드가 있었다"는 사실까지 잃을 수는 없다(시간은 그때 못 적을 뿐이다).
const touchRound = (
  spans: Record<number, NestedSpan>,
  round: number,
  stage: StageKey,
  id: number,
  at: number | null,
): Record<number, NestedSpan> => {
  const entry = spans[round];
  if (entry === undefined) return { ...spans, [round]: { id, stage, firstAt: at, lastAt: at } };
  if (at === null) return spans;
  return { ...spans, [round]: { ...entry, firstAt: entry.firstAt ?? at, lastAt: at } };
};

export const minutesBetween = (from: number, to: number): number => Math.max(0, Math.floor((to - from) / 60_000));

// turn.completed의 usage(§6.2)를 봉투 좌표의 에이전트로 접는다. usage가 없거나(벤더 미보고) 수치가 어긋난
// 이벤트는 누적하지 않는다 — 틀린 수를 더하는 것보다 빠뜨리는 편이 낫고, 표시가 하한임은 화면이 밝힌다.
const accumulateUsage = (
  usage: Record<string, AgentUsage>,
  context: FrameContext,
  payload: Record<string, unknown> | null,
): Record<string, AgentUsage> => {
  if (context.agent === undefined || !isRecord(payload?.usage)) return usage;
  const agent = str(context.agent.name);
  const inputTokens = int(payload.usage.inputTokens);
  const outputTokens = int(payload.usage.outputTokens);
  const cacheReadTokens = int(payload.usage.cacheReadTokens);
  const cacheWriteTokens = int(payload.usage.cacheWriteTokens);
  if (agent === null || inputTokens === null || outputTokens === null || cacheReadTokens === null || cacheWriteTokens === null) {
    return usage;
  }
  const prev = usage[agent] ?? { turns: 0, inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0 };
  return {
    ...usage,
    [agent]: {
      turns: prev.turns + 1,
      inputTokens: prev.inputTokens + inputTokens,
      outputTokens: prev.outputTokens + outputTokens,
      cacheReadTokens: prev.cacheReadTokens + cacheReadTokens,
      cacheWriteTokens: prev.cacheWriteTokens + cacheWriteTokens,
    },
  };
};

// 도구 이름·대상 경로 → 화면 동사. 실패한 호출(ok !== true)은 캡슐감이 아니다 — 재시도 루프가 ×N을 부풀린다.
const markVerb = (payload: Record<string, unknown> | null): ToolVerb | null => {
  if (payload?.ok !== true) return null;
  const tool = str(payload.tool);
  // scratch/는 에이전트 전용 임시 공간 — 산출물 노트와 갈라 "임시 노트"로 부른다
  const path = isRecord(payload.input) ? str(payload.input.path) : null;
  const scratch = path !== null && path.startsWith('scratch/');
  if (tool === 'read') {
    if (path !== null && path.startsWith('manuscript/')) return 'read-manuscript';
    return scratch ? 'read-scratch' : 'read-note';
  }
  if (tool === 'grep') return 'grep';
  if (tool === 'websearch') return 'search';
  if (tool === 'write') return scratch ? 'write-scratch' : 'write';
  if (tool === 'edit') return scratch ? 'edit-scratch' : 'edit';
  return null;
};

// write의 자수 — 사영(frames.ts)은 content 길이를 chars로 남기고, 사영 전 봉투는 content 원문을 싣는다.
const writeChars = (input: Record<string, unknown> | null): number | null =>
  int(input?.chars) ?? (typeof input?.content === 'string' ? input.content.length : null);

export const applyEvent = (state: LiveState, event: SseEvent): LiveState => {
  if (event.id !== null && event.id <= state.cursor) return state;
  const cursor = event.id ?? state.cursor;
  const { payload, context, at } = decode(event.data);
  const kind = consumedKind(event.event);
  if (kind === null) return { ...state, cursor };

  if (kind === 'workflow.started') return { ...state, startedAt: state.startedAt ?? at, cursor };

  // 턴 확정 텍스트가 발화 피드의 유일한 원천이다 — workflow 종결이 아니므로 터미널 판정보다 앞에서 걸러낸다.
  // 도구만 부르고 끝난 턴은 text가 null이고 아무것도 남기지 않는다 — 그 사이의 진행 시각은 tool.executed가 이미 찍었다.
  if (kind === 'turn.completed') {
    // usage 누적은 발화 필터보다 앞이다 — 도구만 부르고 끝난 턴(text null)도 과금은 확정됐다.
    const usage = accumulateUsage(state.usage, context, payload);
    const text = str(payload?.text);
    if (text === null) return { ...state, usage, cursor };
    const timing = state.currentStage === null ? state.timing : touch(state.timing, state.currentStage, at);
    return {
      ...state,
      usage,
      activity: append(state.activity, { id: cursor, text, stage: state.currentStage, step: state.currentStep, at }),
      timing,
      cursor,
    };
  }

  // 파킹은 화면에 질문 카드를 세우는 유일한 신호다 — 종결 판정보다 앞에서 걸러낸다.
  if (kind === 'tool.requested') {
    const parsed = parseAskUser(context, payload);
    if (parsed === null) return { ...state, cursor };
    const timing = state.currentStage === null ? state.timing : touch(state.timing, state.currentStage, at);
    return {
      ...state,
      questions: [
        ...state.questions,
        {
          id: cursor,
          ...parsed,
          stage: state.currentStage,
          step: state.currentStep,
          at,
          settledAt: null,
          status: 'pending',
          answers: null,
        },
      ],
      timing,
      cursor,
    };
  }

  if (TERMINAL_EVENTS.has(kind)) {
    // 종결은 답을 기다리던 질문의 끝이기도 하다 — 답할 수 없는 카드를 대기 상태로 남겨 둘 수 없다.
    const questions = state.questions.map((q) => (q.status === 'pending' ? { ...q, status: 'closed' as const, settledAt: at } : q));
    // 완료는 진행 중이던 스테이지를 닫는다 — 실패·취소는 멈춘 자리를 남겨야 하므로 닫지 않는다.
    const stage = state.currentStage;
    // 돌던 라운드는 어느 종결이든 그 시각에서 끝난다 — 스테이지와 달리 멈춘 자리를 되살릴 여지가 없다.
    const round = nestedRound(state.currentStep);
    const nestedSpans = round === null || stage === null ? state.nestedSpans : touchRound(state.nestedSpans, round, stage, cursor, at);
    // 실패 시각은 재개가 닫을 파킹 구간의 시작점이다 — 시각을 잃은 봉투는 감산을 포기한다(잘못 재기보다 안 재기).
    const failedAt = kind === 'workflow.failed' ? at : state.failedAt;
    if (stage === null || kind !== 'workflow.completed' || state.stages[stage] !== 'running') {
      return { ...state, questions, nestedSpans, failedAt, terminal: true, cursor };
    }
    return {
      ...state,
      stages: { ...state.stages, [stage]: 'done' },
      timing: touch(state.timing, stage, at),
      questions,
      nestedSpans,
      failedAt,
      terminal: true,
      cursor,
    };
  }

  // 재개는 실패 종결의 되돌림이다(workflow.retried — §6.4) — 실패로 굳은 terminal을 풀어야 종결 전이 감지가 다음
  // 종결에서 다시 발화한다. 실패~재개 구간은 리뷰가 돈 시간이 아니라 파킹으로 접는다(질문 대기와 같은 감산 축).
  // 멈춘 스테이지는 실패가 닫지 않았으므로 그대로 이어진다.
  if (kind === 'workflow.retried') {
    const pauses = at !== null && state.failedAt !== null ? [...state.pauses, { at: state.failedAt, settledAt: at }] : state.pauses;
    return { ...state, pauses, failedAt: null, terminal: false, cursor };
  }

  if (kind === 'step.started') {
    const step = str(context.step);
    const stage = step === null ? null : stepStage(step);
    if (step === null || stage === null) return { ...state, cursor };

    const stages = { ...state.stages };
    let timing = state.timing;

    for (const entry of STAGES) {
      if (entry.key === stage) break;
      // 실제로 돌던 스테이지만 종료 시각을 남긴다 — 건너뛴 스테이지에 시각을 찍는 것은 거짓말이다.
      if (stages[entry.key] === 'running') timing = touch(timing, entry.key, at);
      stages[entry.key] = 'done';
    }

    stages[stage] = 'running';
    timing = touch(timing, stage, at);

    // 돌던 라운드는 다음 스텝이 시작하는 순간 닫힌다 — 같은 라운드의 하위 스텝이면 닫자마자 같은 시각으로 다시 열린다.
    let nestedSpans = state.nestedSpans;
    const previous = nestedRound(state.currentStep);
    if (previous !== null && state.currentStage !== null) {
      nestedSpans = touchRound(nestedSpans, previous, state.currentStage, cursor, at);
    }
    const round = nestedRound(step);
    if (round !== null) nestedSpans = touchRound(nestedSpans, round, stage, cursor, at);

    // 스텝 이름은 발화의 귀속처다 — 같은 스테이지 안에서 갈라지는 검수 왕복은 스테이지가 아니라 스텝으로만 구분된다.
    return { ...state, stages, timing, nestedSpans, currentStage: stage, currentStep: step, cursor };
  }

  if (kind === 'step.completed') {
    const step = str(context.step);
    const stage = step === null ? null : stepStage(step);
    if (stage === null) return { ...state, cursor };
    return { ...state, timing: touch(state.timing, stage, at), cursor };
  }

  // 해소는 파킹을 낳은 호출의 toolCallId로 짝짓는다(§6.3 — 같은 좌표). ok:false(해소 체인의 오류 문면 커밋)도
  // 대기의 끝이므로 굳힌다. 짝 없는 해소는 무시된다.
  if (kind === 'tool.resolved') {
    const toolCallId = str(context.toolCallId);
    if (toolCallId === null || state.questions.every((q) => q.status !== 'pending' || q.toolCallId !== toolCallId)) {
      return { ...state, cursor };
    }
    const answers = parseAskAnswers(payload?.data);
    const questions = state.questions.map((q) =>
      q.status === 'pending' && q.toolCallId === toolCallId ? { ...q, status: 'answered' as const, settledAt: at, answers } : q,
    );
    const timing = state.currentStage === null ? state.timing : touch(state.timing, state.currentStage, at);
    return { ...state, questions, timing, cursor };
  }

  if (kind === 'tool.executed') {
    // 호출은 발화가 아니라 활동이다 — 라인 대신 캡슐 자취와 진행 시각을 남긴다.
    if (state.currentStage === null) return { ...state, cursor };
    const timing = touch(state.timing, state.currentStage, at);
    const verb = markVerb(payload);
    if (verb === null) return { ...state, timing, cursor };
    const input = isRecord(payload?.input) ? payload.input : null;
    const mark: ToolMark = {
      id: cursor,
      verb,
      query: verb === 'search' ? str(input?.query) : null,
      chars: verb === 'write' ? writeChars(input) : null,
      stage: state.currentStage,
      step: state.currentStep,
      at,
    };
    return { ...state, timing, marks: [...state.marks, mark].slice(-MARKS_LIMIT), cursor };
  }

  return { ...state, cursor };
};

// 스테이지 본문은 발화 줄·활동 캡슐·검수 라운드 카드가 섞여 흐른다. 세 흐름(라운드 자취·발화·도구 자취)을
// 같은 커서 눈금에 세워 순서를 정하고, 발화 사이의 연속한 도구 자취는 캡슐 하나로 접는다. 항목 사이의 조용한
// 틈(5초 이상)은 "생각"으로 적는다 — 계측이 아니라 이벤트 시각의 근사이고, 도구 실행·문장 생성 시간도 섞인다.
// 입력이 상태뿐이라 새로고침·재생분도 같은 모양으로 다시 선다.
export type CapsuleItem =
  { kind: 'tool'; verb: ToolVerb; count: number; query: string | null; chars: number | null } | { kind: 'think'; seconds: number };

export type FeedEntry =
  | { kind: 'line'; key: string; line: ActivityLine }
  | { kind: 'capsule'; key: string; items: CapsuleItem[] }
  | { kind: 'question'; key: string; entry: QuestionEntry };

export type FeedGroup = FeedEntry | { kind: 'nested'; key: string; round: number; span: NestedSpan | null; feed: FeedEntry[] };

const THINK_MIN_SECONDS = 5;

// 시간 문구 — 60초를 넘으면 분·초로 갈아탄다("100초"는 사람의 자가 아니다). 라이브 카운터와 기록 캡슐이
// 같은 함수를 써서 흐르던 숫자가 그대로 기록으로 굳는다.
export const durationLabel = (seconds: number): string => {
  if (seconds < 60) return `${seconds}초`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return rest === 0 ? `${minutes}분` : `${minutes}분 ${rest}초`;
};

// 캡슐 문구 — 활동을 사용자 어휘로 옮긴다. 결정을 한곳에 모아 바깥 피드와 검수 카드가 같은 말을 쓴다.
// 문구 체계: 기록(여기)은 완료형 축약(…음/…함), 라이브 꼬리는 해요체 진행형 + " · 값"(StageTimeline) —
// 진행 화면의 두 어휘 층이고, 층 안에서는 형태를 섞지 않는다.
export const capsuleLabel = (item: CapsuleItem): string => {
  if (item.kind === 'think') return `${durationLabel(item.seconds)} 생각함`;
  const times = item.count > 1 ? ` ×${item.count}` : '';
  if (item.verb === 'read-manuscript') return `원고 읽음${times}`;
  if (item.verb === 'read-note') return `노트 읽음${times}`;
  if (item.verb === 'read-scratch') return `임시 노트 읽음${times}`;
  if (item.verb === 'grep') return `원고 내 검색함${times}`;
  if (item.verb === 'search') return item.query === null ? '웹 검색함' : `‘${item.query}’ 검색함`;
  if (item.verb === 'write') return item.chars === null ? '노트 작성함' : `노트 ${item.chars.toLocaleString('ko-KR')}자 작성함`;
  if (item.verb === 'write-scratch')
    return item.chars === null ? '임시 노트 작성함' : `임시 노트 ${item.chars.toLocaleString('ko-KR')}자 작성함`;
  if (item.verb === 'edit-scratch') return `임시 노트 고침${times}`;
  return `노트 고침${times}`;
};

export const groupFeed = (state: LiveState, stage: StageKey): FeedGroup[] => {
  type Entry =
    | { id: number; round: number | null; kind: 'span'; span: NestedSpan }
    | { id: number; round: number | null; kind: 'line'; line: ActivityLine }
    | { id: number; round: number | null; kind: 'mark'; mark: ToolMark }
    | { id: number; round: number | null; kind: 'question'; question: QuestionEntry };

  // 같은 커서에 자취와 발화가 겹치면 자취가 앞이다 — 카드를 먼저 열어야 발화가 그 안으로 들어간다.
  const entries: Entry[] = [
    ...Object.entries(state.nestedSpans)
      .filter(([, span]) => span.stage === stage)
      .map(([round, span]) => ({ id: span.id, round: Number(round), kind: 'span' as const, span })),
    ...state.activity
      .filter((line) => line.stage === stage)
      .map((line) => ({ id: line.id, round: nestedRound(line.step), kind: 'line' as const, line })),
    ...state.marks
      .filter((mark) => mark.stage === stage)
      .map((mark) => ({ id: mark.id, round: nestedRound(mark.step), kind: 'mark' as const, mark })),
    ...state.questions
      .filter((q) => q.stage === stage)
      .map((q) => ({ id: q.id, round: nestedRound(q.step), kind: 'question' as const, question: q })),
  ].toSorted((a, b) => a.id - b.id);

  const groups: FeedGroup[] = [];
  let pending: CapsuleItem[] = [];
  let pendingRound: number | null = null;
  let lastAt: number | null = null; // 직전 항목(발화·도구)의 시각 — "생각" 틈의 기준점
  let capsules = 0;

  // 라운드 밖은 최상위, 라운드 안은 그 라운드의 카드가 그릇이다 — 같은 라운드의 연속 구간은 한 카드로 접는다.
  // 최상위 캐스트는 push 전용이라 안전하다: FeedEntry는 FeedGroup의 부분집합이다.
  const container = (round: number | null, span: NestedSpan | null): FeedEntry[] => {
    if (round === null) return groups as FeedEntry[];
    const last = groups.at(-1);
    if (last?.kind === 'nested' && last.round === round) {
      if (span !== null && last.span === null) last.span = span;
      return last.feed;
    }
    const group = { kind: 'nested' as const, key: `nested:${groups.length}:${round}`, round, span, feed: [] as FeedEntry[] };
    groups.push(group);
    return group.feed;
  };

  // 캡슐 확정 시 카테고리로 통합한다 — 인터리브된 낱개 나열은 소음이다. 도구는 동사별 한 항목으로 횟수·자수를
  // 합산하고(첫 등장 순서), 검색은 질의어가 정보라 항목별로 남기며, 생각은 총합 하나를 맨 뒤에 둔다.
  const consolidate = (items: CapsuleItem[]): CapsuleItem[] => {
    const tools: CapsuleItem[] = [];
    const byVerb = new Map<ToolVerb, Extract<CapsuleItem, { kind: 'tool' }>>();
    let thinkSeconds = 0;
    for (const item of items) {
      if (item.kind === 'think') {
        thinkSeconds += item.seconds;
        continue;
      }
      if (item.verb === 'search') {
        tools.push(item);
        continue;
      }
      const merged = byVerb.get(item.verb);
      if (merged === undefined) {
        const copy = { ...item };
        byVerb.set(item.verb, copy);
        tools.push(copy);
      } else {
        merged.count += item.count;
        if (item.chars !== null) merged.chars = (merged.chars ?? 0) + item.chars;
      }
    }
    return thinkSeconds > 0 ? [...tools, { kind: 'think', seconds: thinkSeconds }] : tools;
  };

  const flush = () => {
    if (pending.length === 0) return;
    container(pendingRound, null).push({ kind: 'capsule', key: `capsule:${capsules}`, items: consolidate(pending) });
    capsules += 1;
    pending = [];
  };

  for (const entry of entries) {
    if (entry.kind === 'mark') {
      if (pending.length > 0 && pendingRound !== entry.round) flush();
      pendingRound = entry.round;
      if (lastAt !== null && entry.mark.at !== null) {
        const seconds = Math.round((entry.mark.at - lastAt) / 1000);
        if (seconds >= THINK_MIN_SECONDS) pending.push({ kind: 'think', seconds });
      }
      pending.push({ kind: 'tool', verb: entry.mark.verb, count: 1, query: entry.mark.query, chars: entry.mark.chars });
      if (entry.mark.at !== null) lastAt = entry.mark.at;
      continue;
    }

    if (entry.kind === 'span') {
      flush();
      container(entry.round, entry.span);
      continue;
    }

    // 질문은 생각의 기준점을 끊는다 — 질문 뒤의 틈에는 작가가 답을 고른 시간이 섞여 있고, 사람이 기다린 시간을
    // 모델이 생각한 시간으로 적을 수는 없다. 다음 항목이 스스로 새 기준점을 세운다.
    if (entry.kind === 'question') {
      flush();
      container(entry.round, null).push({ kind: 'question', key: `question:${entry.question.id}`, entry: entry.question });
      lastAt = null;
      continue;
    }

    // 발화 앞의 조용한 틈도 생각이다 — 한 턴의 발화와 도구 호출은 같은 커밋 배치라 시각이 붙어 있고, 생각·문장
    // 생성 시간은 전부 직전 이벤트에서 그 턴의 발화까지의 틈에 놓인다. 여기서 접지 않으면 생각은 기록에서
    // 통째로 사라진다(마크 앞의 틈은 도구만 부르는 턴의 생각을 잡는다).
    if (lastAt !== null && entry.line.at !== null) {
      const seconds = Math.round((entry.line.at - lastAt) / 1000);
      if (seconds >= THINK_MIN_SECONDS) {
        if (pending.length > 0 && pendingRound !== entry.round) flush();
        pendingRound = entry.round;
        pending.push({ kind: 'think', seconds });
      }
    }
    flush();
    container(entry.round, null).push({ kind: 'line', key: `line:${entry.line.id}`, line: entry.line });
    if (entry.line.at !== null) lastAt = entry.line.at;
  }
  flush();

  return groups;
};

export const initialLive = (events: SseEvent[]): LiveState => {
  let state: LiveState = {
    stages: Object.fromEntries(STAGES.map((entry) => [entry.key, 'pending'])) as Record<StageKey, StageStatus>,
    timing: Object.fromEntries(STAGES.map((entry) => [entry.key, { firstAt: null, lastAt: null }])) as Record<StageKey, StageTiming>,
    activity: [],
    marks: [],
    questions: [],
    nestedSpans: {},
    usage: {},
    currentStage: null,
    currentStep: null,
    startedAt: null,
    pauses: [],
    failedAt: null,
    terminal: false,
    cursor: 0,
  };
  for (const event of events) state = applyEvent(state, event);
  return state;
};

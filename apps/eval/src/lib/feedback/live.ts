import { nestedRound, STAGES, stepStage, TERMINAL_EVENTS } from './stages.ts';
import type { SseEvent } from './sse.ts';
import type { StageKey } from './stages.ts';

export type StageStatus = 'pending' | 'running' | 'done';

// 활동 라인은 모델이 턴을 확정하며 남긴 발화다(prism/src/session/loop.ts:378 — turn.completed 이벤트 data에
// text가 실린다). 도구 호출은 라인을 만들지 않고, 기술 요약(경로·좌표)도 화면에 싣지 않는다.
export type ActivityLine = { id: number; text: string; stage: StageKey | null; step: string | null; at: number | null };

export type StageTiming = { firstAt: number | null; lastAt: number | null };

// 검수 라운드는 발화 없이 지나갈 수 있다 — 기계적으로 끝낸 왕복에서는 모델이 입을 열지 않는다. 발화만으로 화면을
// 세우면 그런 라운드는 흔적 없이 사라지므로, 턴과 별개로 스텝 자체의 자취를 라운드마다 접어 둔다. id는 발화 라인과
// 같은 커서라(둘 다 이벤트 순번) 두 흐름을 한 줄에 세울 수 있다.
export type NestedSpan = { id: number; stage: StageKey; firstAt: number | null; lastAt: number | null };

// 도구 호출의 자취 — tool.called의 관측 사영(prism/src/session/tool-dispatch.ts:195-203)에서 화면이 쓸 것만
// 추린다. read는 대상 경로로 원고/노트를 가르고, 실패한 호출과 좌표성 낮은 도구(list)는 남기지 않는다 —
// 캡슐은 활동의 요약이지 감사 로그가 아니다.
export type ToolVerb = 'read-manuscript' | 'read-note' | 'grep' | 'search' | 'write' | 'edit';
export type ToolMark = {
  id: number;
  verb: ToolVerb;
  query: string | null;
  chars: number | null;
  stage: StageKey;
  step: string | null;
  at: number | null;
};

export type LiveState = {
  stages: Record<StageKey, StageStatus>;
  timing: Record<StageKey, StageTiming>;
  activity: ActivityLine[];
  marks: ToolMark[];
  nestedSpans: Record<number, NestedSpan>;
  currentStage: StageKey | null;
  currentStep: string | null;
  startedAt: number | null;
  terminal: boolean;
  cursor: number;
};

const ACTIVITY_LIMIT = 200;
const MARKS_LIMIT = 400;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

const str = (value: unknown): string | null => (typeof value === 'string' && value.length > 0 ? value : null);
const int = (value: unknown): number | null => (typeof value === 'number' && Number.isFinite(value) ? value : null);

// prism SSE의 data 라인은 이벤트 본문이 아니라 {seq,kind,data,createdAt} 봉투다 — createdAt이 UI 시간 표시의
// 원 시각이고(재생분에 수신 시각은 오답), 본문은 한 겹 안에 있다. prism/src/session/sse.ts:47-50.
const decode = (raw: string): { payload: Record<string, unknown> | null; at: number | null } => {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { payload: null, at: null };
  }
  if (!isRecord(parsed)) return { payload: null, at: null };
  return { payload: isRecord(parsed.data) ? parsed.data : null, at: int(parsed.createdAt) };
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

// 도구 이름·대상 경로 → 화면 동사. 실패한 호출(ok !== true)은 캡슐감이 아니다 — 재시도 루프가 ×N을 부풀린다.
const markVerb = (payload: Record<string, unknown> | null): ToolVerb | null => {
  if (payload?.ok !== true) return null;
  const tool = str(payload.tool);
  if (tool === 'read') {
    const path = isRecord(payload.input) ? str(payload.input.path) : null;
    return path !== null && path.startsWith('manuscript/') ? 'read-manuscript' : 'read-note';
  }
  if (tool === 'grep') return 'grep';
  if (tool === 'websearch') return 'search';
  if (tool === 'write') return 'write';
  if (tool === 'edit') return 'edit';
  return null;
};

export const applyEvent = (state: LiveState, event: SseEvent): LiveState => {
  if (event.id !== null && event.id <= state.cursor) return state;
  const cursor = event.id ?? state.cursor;
  const { payload, at } = decode(event.data);

  if (event.event === 'run.started') return { ...state, startedAt: state.startedAt ?? at, cursor };

  // 턴 확정 텍스트가 발화 피드의 유일한 원천이다 — run 종결이 아니므로 터미널 판정보다 앞에서 걸러낸다.
  // 도구만 부르고 끝난 턴은 text가 null이고 아무것도 남기지 않는다 — 그 사이의 진행 시각은 tool.called이 이미 찍었다.
  if (event.event === 'turn.completed') {
    const text = str(payload?.text);
    if (text === null) return { ...state, cursor };
    const timing = state.currentStage === null ? state.timing : touch(state.timing, state.currentStage, at);
    return {
      ...state,
      activity: append(state.activity, { id: cursor, text, stage: state.currentStage, step: state.currentStep, at }),
      timing,
      cursor,
    };
  }

  if (TERMINAL_EVENTS.has(event.event)) {
    // 자식 run의 종결은 루트의 종결이 아니다. 중계되어 루트 로그에 실리는 자식 이벤트에는 agent가 찍히고
    // (prism/src/session/propagation.ts:287), 루트 자신의 종결에는 그 필드가 없다
    // (prism/src/session/terminal.ts:82-84) — 허브의 봉인 판정도 같은 잣대를 쓴다(sse.ts:369). 그래서 agent가
    // 실린 종결은 커서만 밀고 지나간다: 여기서 터미널을 세우면 첫 자식이 끝나는 순간 스트림이 닫히고
    // 화면은 완료에 닿지 못한다. 스테이지도 라운드도 루트의 종결에서만 닫힌다.
    if (payload?.agent !== undefined) return { ...state, cursor };

    // 완료는 진행 중이던 스테이지를 닫는다 — 실패·취소는 멈춘 자리를 남겨야 하므로 닫지 않는다.
    const stage = state.currentStage;
    // 돌던 라운드는 어느 종결이든 그 시각에서 끝난다 — 스테이지와 달리 멈춘 자리를 되살릴 여지가 없다.
    const round = nestedRound(state.currentStep);
    const nestedSpans = round === null || stage === null ? state.nestedSpans : touchRound(state.nestedSpans, round, stage, cursor, at);
    if (stage === null || event.event !== 'run.completed' || state.stages[stage] !== 'running') {
      return { ...state, nestedSpans, terminal: true, cursor };
    }
    return {
      ...state,
      stages: { ...state.stages, [stage]: 'done' },
      timing: touch(state.timing, stage, at),
      nestedSpans,
      terminal: true,
      cursor,
    };
  }

  if (event.event === 'step.started') {
    const step = str(payload?.step);
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

    // 스텝 이름은 발화의 귀속처다 — 같은 스테이지 안에서 갈라지는 계획 왕복은 스테이지가 아니라 스텝으로만 구분된다.
    return { ...state, stages, timing, nestedSpans, currentStage: stage, currentStep: step, cursor };
  }

  if (event.event === 'step.completed') {
    const step = str(payload?.step);
    const stage = step === null ? null : stepStage(step);
    if (stage === null) return { ...state, cursor };
    return { ...state, timing: touch(state.timing, stage, at), cursor };
  }

  if (event.event === 'tool.called') {
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
      chars: verb === 'write' ? int(input?.chars) : null,
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

export type FeedEntry = { kind: 'line'; key: string; line: ActivityLine } | { kind: 'capsule'; key: string; items: CapsuleItem[] };

export type FeedGroup = FeedEntry | { kind: 'nested'; key: string; round: number; span: NestedSpan | null; feed: FeedEntry[] };

const THINK_MIN_SECONDS = 5;

// 같은 동사의 연속만 ×N으로 접는다 — 검색(질의어)과 작성(자 수)은 항목마다 고유한 값이 있어 접지 않는다.
const FOLDABLE = new Set<ToolVerb>(['read-manuscript', 'read-note', 'grep', 'edit']);

// 캡슐 문구 — 활동을 사용자 어휘로 옮긴다. 결정을 한곳에 모아 바깥 피드와 검수 카드가 같은 말을 쓴다.
// 문구 체계: 기록(여기)은 완료형 축약(…음/…함), 라이브 꼬리는 해요체 진행형 + " · 값"(StageTimeline) —
// 진행 화면의 두 어휘 층이고, 층 안에서는 형태를 섞지 않는다.
export const capsuleLabel = (item: CapsuleItem): string => {
  if (item.kind === 'think') return item.seconds < 60 ? `${item.seconds}초 생각함` : `${Math.round(item.seconds / 60)}분 생각함`;
  const times = item.count > 1 ? ` ×${item.count}` : '';
  if (item.verb === 'read-manuscript') return `원고 읽음${times}`;
  if (item.verb === 'read-note') return `노트 읽음${times}`;
  if (item.verb === 'grep') return `원고 내 검색함${times}`;
  if (item.verb === 'search') return item.query === null ? '웹 검색함' : `‘${item.query}’ 검색함`;
  if (item.verb === 'write') return item.chars === null ? '노트 작성함' : `노트 ${item.chars.toLocaleString('ko-KR')}자 작성함`;
  return '노트 고침';
};

export const groupFeed = (state: LiveState, stage: StageKey): FeedGroup[] => {
  type Entry =
    | { id: number; round: number | null; kind: 'span'; span: NestedSpan }
    | { id: number; round: number | null; kind: 'line'; line: ActivityLine }
    | { id: number; round: number | null; kind: 'mark'; mark: ToolMark };

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

  const flush = () => {
    if (pending.length === 0) return;
    container(pendingRound, null).push({ kind: 'capsule', key: `capsule:${capsules}`, items: pending });
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
      const last = pending.at(-1);
      if (last?.kind === 'tool' && last.verb === entry.mark.verb && FOLDABLE.has(entry.mark.verb)) last.count += 1;
      else pending.push({ kind: 'tool', verb: entry.mark.verb, count: 1, query: entry.mark.query, chars: entry.mark.chars });
      if (entry.mark.at !== null) lastAt = entry.mark.at;
      continue;
    }

    if (entry.kind === 'span') {
      flush();
      container(entry.round, entry.span);
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
    nestedSpans: {},
    currentStage: null,
    currentStep: null,
    startedAt: null,
    terminal: false,
    cursor: 0,
  };
  for (const event of events) state = applyEvent(state, event);
  return state;
};

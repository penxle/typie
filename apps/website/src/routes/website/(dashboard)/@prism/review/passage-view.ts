import { collapseRows } from '../lib/tool-calls.ts';
import { currentStep } from '../lib/trace.ts';
import { stagesFor, stepRound, stepStage } from './stages.ts';
import type { PrismReviewTierName } from '@typie/prism';
import type { ToolRequestMessage, WorkflowStatus } from '../lib/conversation.ts';
import type { TurnLive } from '../lib/delta.ts';
import type { ToolRow } from '../lib/tool-calls.ts';
import type { TraceStep, TraceTool, WorkflowTrace } from '../lib/trace.ts';
import type { StageKey } from './stages.ts';

export { type ToolRow } from '../lib/tool-calls.ts';

export type PassageGroup =
  | { kind: 'narration'; key: string; seq: number; text: string }
  | { kind: 'tools'; key: string; seq: number; count: number; rows: ToolRow[] }
  | { kind: 'question'; key: string; seq: number; request: ToolRequestMessage }
  | { kind: 'round'; key: string; seq: number; round: number; elapsedMs: number; groups: PassageGroup[] };

export type StageStatus = 'pending' | 'running' | 'done' | 'canceled' | 'failed';

export type StageView = {
  key: StageKey;
  label: string;
  status: StageStatus;
  elapsedMs: number | null;
  summary: string | null;
  rounds: number;
  groups: PassageGroup[];
};

export type TailLabel = '생각하는 중' | '메모를 쓰는 중' | '메모를 고치는 중' | '다시 연결하는 중' | '응답이 늦어지고 있어요' | null;

export type PassageView = {
  prelude: PassageGroup[];
  stages: StageView[];
  current: StageKey | null;
  liveRound: number | null;
  elapsedMs: number;
};

export const LATE_MS = 30_000;

export const toolRowLabel = (tool: TraceTool): string | null => {
  if (!tool.ok) return null;
  if (tool.tool === 'read') return tool.path?.startsWith('manuscript/') ? '원고를 읽었어요' : '메모를 읽었어요';
  if (tool.tool === 'grep') return '원고에서 찾아봤어요';
  if (tool.tool === 'write') return '메모를 남겼어요';
  if (tool.tool === 'edit') return '메모를 고쳤어요';
  if (tool.tool === 'websearch') return tool.query === null ? '웹에서 찾아봤어요' : `웹에서 ‘${tool.query}’을 찾아봤어요`;
  return null;
};

const TOOL_INPUT_LABEL: Record<string, TailLabel> = {
  write: '메모를 쓰는 중',
  edit: '메모를 고치는 중',
};

export const tailLabel = ({ live, reconnecting, lateMs }: { live: TurnLive | null; reconnecting: boolean; lateMs: number }): TailLabel => {
  if (reconnecting) return '다시 연결하는 중';
  if (lateMs >= LATE_MS) return '응답이 늦어지고 있어요';
  if (live === null || live.last === 'text') return null;
  if (live.last === 'tool.input') return live.toolInput === null ? null : (TOOL_INPUT_LABEL[live.toolInput.name] ?? null);
  return live.thinkingChars > 0 ? '생각하는 중' : null;
};

const MINUTE = 60_000;

export const runningLabel = (ms: number): string => (ms < MINUTE ? '방금' : `${Math.floor(ms / MINUTE)}분째`);
export const spentLabel = (ms: number): string => (ms < MINUTE ? '1분 미만' : `${Math.floor(ms / MINUTE)}분`);
export const elapsedLabel = (ms: number): string => (ms < MINUTE ? '방금 시작' : `${Math.floor(ms / MINUTE)}분`);

type Span = { from: number; to: number | null };

const overlap = (spans: Span[], from: number, to: number): number => {
  let total = 0;
  for (const span of spans) {
    const start = Math.max(span.from, from);
    const end = Math.min(span.to ?? to, to);
    if (end > start) total += end - start;
  }
  return total;
};

const waitSpans = (requests: ToolRequestMessage[]): Span[] => {
  const sorted = requests
    .map((request) => ({ from: request.at, to: request.status === 'pending' ? null : (request.settledAt ?? request.at) }))
    .toSorted((a, b) => a.from - b.from);

  const merged: Span[] = [];
  for (const span of sorted) {
    const last = merged.at(-1);
    if (last === undefined || (last.to !== null && span.from > last.to)) {
      merged.push({ ...span });
      continue;
    }
    if (last.to !== null) last.to = span.to === null ? null : Math.max(last.to, span.to);
  }
  return merged;
};

type Item =
  | { kind: 'turn'; seq: number; step: string | null; text: string }
  | { kind: 'tool'; seq: number; step: string | null; tool: TraceTool }
  | { kind: 'question'; seq: number; step: string | null; request: ToolRequestMessage };

type RoundSpan = { from: number; to: number | null; seq: number; open: boolean };

type StepIndex = {
  stageOf: Map<string, StageKey | null>;
  firstStart: Map<StageKey, number>;
  roundsOf: Map<StageKey, Set<number>>;
  roundSpans: Map<string, RoundSpan>;
};

const roundKey = (stage: StageKey, round: number): string => `${stage}|${round}`;

const indexSteps = (steps: TraceStep[]): StepIndex => {
  const stageOf = new Map<string, StageKey | null>();
  const firstStart = new Map<StageKey, number>();
  const roundsOf = new Map<StageKey, Set<number>>();
  const roundSpans = new Map<string, RoundSpan>();
  let carried: StageKey | null = null;

  for (const step of steps) {
    const direct = stepStage(step.name);
    const stage = direct ?? carried;
    stageOf.set(step.name, stage);

    if (direct !== null) {
      carried = direct;
      if (!firstStart.has(direct)) firstStart.set(direct, step.startedAt);
    }

    const round = stepRound(step.name);
    if (stage === null || round === null) continue;

    const rounds = roundsOf.get(stage);
    if (rounds === undefined) roundsOf.set(stage, new Set([round]));
    else rounds.add(round);

    const key = roundKey(stage, round);
    const span = roundSpans.get(key);
    if (span === undefined) {
      roundSpans.set(key, { from: step.startedAt, to: step.completedAt, seq: step.seq, open: step.completedAt === null });
      continue;
    }
    span.to = step.completedAt ?? span.to;
    span.open ||= step.completedAt === null;
  }

  return { stageOf, firstStart, roundsOf, roundSpans };
};

const lastKnownAt = (trace: WorkflowTrace): number => {
  let latest = 0;
  for (const step of trace.steps) latest = Math.max(latest, step.completedAt ?? step.startedAt);
  for (const turn of trace.turns) latest = Math.max(latest, turn.at);
  for (const tool of trace.tools) latest = Math.max(latest, tool.at);
  return latest;
};

const groupItems = (items: Item[], keyPrefix: string): PassageGroup[] => {
  const groups: PassageGroup[] = [];
  let labels: string[] = [];
  let firstSeq = 0;

  const flush = () => {
    if (labels.length === 0) return;
    groups.push({ kind: 'tools', key: `${keyPrefix}:tools:${firstSeq}`, seq: firstSeq, count: labels.length, rows: collapseRows(labels) });
    labels = [];
  };

  for (const item of items) {
    if (item.kind === 'tool') {
      const label = toolRowLabel(item.tool);
      if (label === null) continue;
      if (labels.length === 0) firstSeq = item.seq;
      labels.push(label);
      continue;
    }
    flush();
    if (item.kind === 'turn') groups.push({ kind: 'narration', key: `${keyPrefix}:turn:${item.seq}`, seq: item.seq, text: item.text });
    else groups.push({ kind: 'question', key: `${keyPrefix}:question:${item.seq}`, seq: item.seq, request: item.request });
  }
  flush();
  return groups;
};

const roundOf = (step: string | null): number | null => (step === null ? null : stepRound(step));

const buildGroups = (index: StepIndex, items: Item[], stage: StageKey, waits: Span[], endAt: number): PassageGroup[] => {
  const groups: PassageGroup[] = [];
  let plain: Item[] = [];
  let roundItems: Item[] = [];
  let currentRound: number | null = null;
  let containers = 0;

  const flushPlain = () => {
    groups.push(...groupItems(plain, stage));
    plain = [];
  };

  const flushRound = () => {
    if (currentRound === null) return;
    const span = index.roundSpans.get(roundKey(stage, currentRound));
    const from = span?.from ?? 0;
    const to = span === undefined || span.open ? endAt : (span.to ?? endAt);
    const key = `${stage}:round:${currentRound}:${containers}`;
    groups.push({
      kind: 'round',
      key,
      seq: span?.seq ?? 0,
      round: currentRound,
      elapsedMs: Math.max(0, to - from - overlap(waits, from, to)),
      groups: groupItems(roundItems, key),
    });
    containers += 1;
    roundItems = [];
    currentRound = null;
  };

  for (const item of items) {
    const round = roundOf(item.step);
    if (round === null) {
      flushRound();
      plain.push(item);
      continue;
    }
    if (currentRound !== null && currentRound !== round) flushRound();
    if (currentRound === null) {
      flushPlain();
      currentRound = round;
    }
    roundItems.push(item);
  }
  flushRound();
  flushPlain();

  return groups;
};

export const buildPassage = ({
  trace,
  status,
  tier,
  requests,
  now,
  finishedAt,
}: {
  trace: WorkflowTrace;
  status: WorkflowStatus;
  tier: PrismReviewTierName;
  requests: ToolRequestMessage[];
  now: number;
  finishedAt: number | null;
}): PassageView => {
  const order = stagesFor(tier);
  const waits = waitSpans(requests);
  const index = indexSteps(trace.steps);
  const endAt = status === 'running' ? now : (finishedAt ?? lastKnownAt(trace));

  const items: Item[] = [
    ...trace.turns.map((turn): Item => ({ kind: 'turn', seq: turn.seq, step: turn.step, text: turn.text })),
    ...trace.tools.map((tool): Item => ({ kind: 'tool', seq: tool.seq, step: tool.step, tool })),
    ...requests.map((request): Item => ({ kind: 'question', seq: request.seq, step: null, request })),
  ].toSorted((a, b) => a.seq - b.seq);

  let current: StageKey | null = null;
  for (let i = trace.steps.length - 1; i >= 0; i -= 1) {
    const stage = index.stageOf.get(trace.steps[i].name) ?? null;
    if (stage !== null && order.some((entry) => entry.key === stage)) {
      current = stage;
      break;
    }
  }

  const buckets = new Map<StageKey, Item[]>();
  const preludeItems: Item[] = [];
  let cursor = 0;
  let priorStep: string | null = null;

  const bucketOf = (stage: StageKey, item: Item) => {
    const bucket = buckets.get(stage);
    if (bucket === undefined) buckets.set(stage, [item]);
    else bucket.push(item);
  };

  for (const item of items) {
    while (cursor < trace.steps.length && trace.steps[cursor].seq < item.seq) {
      priorStep = trace.steps[cursor].name;
      cursor += 1;
    }
    if (item.kind === 'question') item.step = priorStep;

    const stage = item.step === null ? null : (index.stageOf.get(item.step) ?? null);

    if (item.kind === 'question' && (stage === null || order.every((entry) => entry.key !== stage))) {
      if (current === null) preludeItems.push(item);
      else bucketOf(current, item);
      continue;
    }

    if (stage === null) continue;
    bucketOf(stage, item);
  }

  const terminal = status === 'canceled' || status === 'failed';
  const currentIndex = current === null ? (terminal ? 0 : -1) : order.findIndex((stage) => stage.key === current);

  const nextStarts: (number | undefined)[] = Array.from({ length: order.length });
  let laterStart: number | undefined;
  for (let i = order.length - 1; i >= 0; i -= 1) {
    nextStarts[i] = laterStart;
    const start = index.firstStart.get(order[i].key);
    if (start !== undefined) laterStart = start;
  }

  const stages = order.map((stage, position): StageView => {
    const stageItems = buckets.get(stage.key) ?? [];
    const from = index.firstStart.get(stage.key) ?? null;
    const to = nextStarts[position] ?? endAt;
    const lastTurn = stageItems.findLast((item) => item.kind === 'turn');

    let stageStatus: StageStatus = 'pending';
    if (position < currentIndex) stageStatus = 'done';
    else if (position === currentIndex) {
      stageStatus = status === 'running' ? 'running' : status === 'completed' ? 'done' : status;
    }

    return {
      key: stage.key,
      label: stage.label,
      status: stageStatus,
      elapsedMs: from === null || stageStatus === 'pending' ? null : Math.max(0, to - from - overlap(waits, from, to)),
      summary: lastTurn?.text.split('\n')[0] ?? null,
      rounds: index.roundsOf.get(stage.key)?.size ?? 0,
      groups: buildGroups(index, stageItems, stage.key, waits, endAt),
    };
  });

  const liveRound = status === 'running' ? roundOf(currentStep(trace)) : null;

  if (liveRound !== null && current !== null) {
    const stage = stages.find((entry) => entry.key === current);

    if (stage !== undefined && stage.groups.every((group) => group.kind !== 'round' || group.round !== liveRound)) {
      const span = index.roundSpans.get(roundKey(current, liveRound));
      const from = span?.from ?? endAt;
      const containers = stage.groups.filter((group) => group.kind === 'round').length;

      stage.groups.push({
        kind: 'round',
        key: `${current}:round:${liveRound}:${containers}`,
        seq: span?.seq ?? 0,
        round: liveRound,
        elapsedMs: Math.max(0, endAt - from - overlap(waits, from, endAt)),
        groups: [],
      });
    }
  }

  const startedAt = trace.steps[0]?.startedAt ?? null;
  const elapsedMs = startedAt === null ? 0 : Math.max(0, endAt - startedAt - overlap(waits, startedAt, endAt));

  return {
    prelude: groupItems(preludeItems, 'prelude'),
    stages,
    current,
    liveRound,
    elapsedMs,
  };
};

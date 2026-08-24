import { match } from 'ts-pattern';
import type { RunState, Transcript, TranscriptMessage, WorkflowStatus } from './conversation.ts';
import type { WorkflowTranscript } from './workflow-transcript.ts';

export type RunStateWire = 'RUNNING' | 'COMPLETED' | 'FAILED' | 'CANCELED';
export type TurnStateWire = 'IDLE' | 'ACTIVE';

export type WorkflowTranscriptWire = {
  steps: { name: string; seq: number; startedAt: string; completedAt: string | null }[];
  turns: { seq: number; step: string | null; text: string; at: string }[];
  tools: { seq: number; step: string | null; tool: string; ok: boolean; path: string | null; query: string | null; at: string }[];
};

export type RunItemWire =
  | { kind: 'user'; key: string; text: string; at: string }
  | { kind: 'assistant'; key: string; text: string | null; toolCalls: { id: string; name: string }[]; at: string; streamed: boolean }
  | { kind: 'tool'; key: string; name: string; phase: 'EXECUTED' | 'REJECTED'; ok: boolean | null; at: string }
  | {
      kind: 'toolRequest';
      key: string;
      seq: number;
      tool: string;
      toolCallId: string;
      agentId: string;
      workflowId: string | null;
      data: unknown;
      status: 'PENDING' | 'RESOLVED' | 'CLOSED';
      result: unknown;
      settledAt: string | null;
      at: string;
    }
  | {
      kind: 'workflow';
      key: string;
      prismWorkflowId: string;
      app: string;
      name: string;
      status: RunStateWire;
      startedAt: string;
      finishedAt: string | null;
      cursor: number;
      invocation: string | null;
      transcript: WorkflowTranscriptWire;
    }
  | { kind: 'runFailure'; key: string; at: string };

export type RunItemsWire = { runSeq: number | null; items: RunItemWire[] };
export type RunWire = RunItemsWire & { state: RunStateWire };
export type TranscriptWire<R = RunWire> = {
  cursor: number;
  title: string | null;
  agentId: string | null;
  turn: TurnStateWire;
  retrying: boolean;
  runs: R[];
};

const iso = (ms: number): string => new Date(ms).toISOString();
const isoOrNull = (ms: number | undefined): string | null => (ms === undefined ? null : iso(ms));
const ms = (value: string): number => Date.parse(value);

const WORKFLOW_STATUS_TO_WIRE: Record<WorkflowStatus, RunStateWire> = {
  running: 'RUNNING',
  completed: 'COMPLETED',
  failed: 'FAILED',
  canceled: 'CANCELED',
};
const WIRE_TO_WORKFLOW_STATUS: Record<RunStateWire, WorkflowStatus> = {
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  CANCELED: 'canceled',
};
const WIRE_TO_RUN_STATE: Record<RunStateWire, RunState> = { RUNNING: 'running', COMPLETED: 'idle', FAILED: 'failed', CANCELED: 'canceled' };

const workflowTranscriptToWire = (transcript: WorkflowTranscript): WorkflowTranscriptWire => ({
  steps: transcript.steps.map((step) => ({
    name: step.name,
    seq: step.seq,
    startedAt: iso(step.startedAt),
    completedAt: step.completedAt === null ? null : iso(step.completedAt),
  })),
  turns: transcript.turns.map((turn) => ({ seq: turn.seq, step: turn.step, text: turn.text, at: iso(turn.at) })),
  tools: transcript.tools.map((tool) => ({
    seq: tool.seq,
    step: tool.step,
    tool: tool.tool,
    ok: tool.ok,
    path: tool.path,
    query: tool.query,
    at: iso(tool.at),
  })),
});

const workflowTranscriptFromWire = (wire: WorkflowTranscriptWire): WorkflowTranscript => ({
  steps: wire.steps.map((step) => ({
    name: step.name,
    seq: step.seq,
    startedAt: ms(step.startedAt),
    completedAt: step.completedAt === null ? null : ms(step.completedAt),
  })),
  turns: wire.turns.map((turn) => ({ seq: turn.seq, step: turn.step, text: turn.text, at: ms(turn.at) })),
  tools: wire.tools.map((tool) => ({
    seq: tool.seq,
    step: tool.step,
    tool: tool.tool,
    ok: tool.ok,
    path: tool.path,
    query: tool.query,
    at: ms(tool.at),
  })),
  live: null,
});

const itemToWire = (message: TranscriptMessage): RunItemWire =>
  match(message)
    .with({ role: 'user' }, (m) => ({ kind: 'user' as const, key: m.key, text: m.text, at: iso(m.at) }))
    .with({ role: 'assistant' }, (m) => ({
      kind: 'assistant' as const,
      key: m.key,
      text: m.text,
      toolCalls: m.toolCalls,
      at: iso(m.at),
      streamed: m.streamed,
    }))
    .with({ role: 'tool' }, (m) => ({
      kind: 'tool' as const,
      key: m.key,
      name: m.name,
      phase: m.phase === 'executed' ? ('EXECUTED' as const) : ('REJECTED' as const),
      ok: m.ok,
      at: iso(m.at),
    }))
    .with({ role: 'tool-request' }, (m) => ({
      kind: 'toolRequest' as const,
      key: m.key,
      seq: m.seq,
      tool: m.tool,
      toolCallId: m.toolCallId,
      agentId: m.agentId,
      workflowId: m.workflowId ?? null,
      data: m.data ?? null,
      status: m.status === 'pending' ? ('PENDING' as const) : m.status === 'resolved' ? ('RESOLVED' as const) : ('CLOSED' as const),
      result: m.result ?? null,
      settledAt: isoOrNull(m.settledAt),
      at: iso(m.at),
    }))
    .with({ role: 'workflow' }, (m) => ({
      kind: 'workflow' as const,
      key: m.key,
      prismWorkflowId: m.workflowId,
      app: m.app,
      name: m.name,
      status: WORKFLOW_STATUS_TO_WIRE[m.status],
      startedAt: iso(m.startedAt),
      finishedAt: isoOrNull(m.finishedAt),
      cursor: m.cursor,
      invocation: m.invocation ?? null,
      transcript: workflowTranscriptToWire(m.transcript),
    }))
    .with({ role: 'run-failed' }, (m) => ({ kind: 'runFailure' as const, key: m.key, at: iso(m.at) }))
    .exhaustive();

const itemFromWire = (item: RunItemWire, runSeq: number | null): TranscriptMessage =>
  match(item)
    .with({ kind: 'user' }, (i) => ({ role: 'user' as const, key: i.key, text: i.text, at: ms(i.at), runSeq }))
    .with({ kind: 'assistant' }, (i) => ({
      role: 'assistant' as const,
      key: i.key,
      text: i.text,
      toolCalls: i.toolCalls,
      at: ms(i.at),
      streamed: i.streamed,
    }))
    .with({ kind: 'tool' }, (i) => ({
      role: 'tool' as const,
      key: i.key,
      name: i.name,
      phase: i.phase === 'EXECUTED' ? ('executed' as const) : ('rejected' as const),
      ok: i.ok,
      at: ms(i.at),
    }))
    .with({ kind: 'toolRequest' }, (i) => ({
      role: 'tool-request' as const,
      key: i.key,
      seq: i.seq,
      tool: i.tool,
      toolCallId: i.toolCallId,
      agentId: i.agentId,
      ...(i.workflowId !== null && { workflowId: i.workflowId }),
      data: i.data,
      status: i.status === 'PENDING' ? ('pending' as const) : i.status === 'RESOLVED' ? ('resolved' as const) : ('closed' as const),
      ...(i.result !== null && { result: i.result }),
      ...(i.settledAt !== null && { settledAt: ms(i.settledAt) }),
      at: ms(i.at),
    }))
    .with({ kind: 'workflow' }, (i) => ({
      role: 'workflow' as const,
      key: i.key,
      workflowId: i.prismWorkflowId,
      app: i.app,
      name: i.name,
      status: WIRE_TO_WORKFLOW_STATUS[i.status],
      startedAt: ms(i.startedAt),
      ...(i.finishedAt !== null && { finishedAt: ms(i.finishedAt) }),
      cursor: i.cursor,
      ...(i.invocation !== null && { invocation: i.invocation }),
      transcript: workflowTranscriptFromWire(i.transcript),
    }))
    .with({ kind: 'runFailure' }, (i) => ({ role: 'run-failed' as const, key: i.key, at: ms(i.at) }))
    .exhaustive();

export const toGraphQL = (t: Transcript): TranscriptWire<RunItemsWire> => {
  const runs: RunItemsWire[] = [];
  for (const message of t.messages) {
    if (message.role === 'user' || runs.length === 0) runs.push({ runSeq: message.role === 'user' ? message.runSeq : null, items: [] });
    runs.at(-1)?.items.push(itemToWire(message));
  }

  return {
    cursor: t.cursor,
    title: t.title,
    agentId: t.agentId,
    turn: t.turn === 'active' ? 'ACTIVE' : 'IDLE',
    retrying: t.retrying,
    runs,
  };
};

export const fromGraphQL = (wire: TranscriptWire): Transcript => {
  const last = wire.runs.at(-1);

  return {
    messages: wire.runs.flatMap((run) => run.items.map((item) => itemFromWire(item, run.runSeq))),
    run: last === undefined ? 'idle' : WIRE_TO_RUN_STATE[last.state],
    turn: wire.turn === 'ACTIVE' ? 'active' : 'idle',
    retrying: wire.retrying,
    live: null,
    cursor: wire.cursor,
    agentId: wire.agentId,
    title: wire.title,
  };
};

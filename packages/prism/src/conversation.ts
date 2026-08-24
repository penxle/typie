import { match, P } from 'ts-pattern';
import { applyDelta, sealTurn, startTurn } from './delta.ts';
import { applyWorkflowTranscriptDelta, applyWorkflowTranscriptEvent, emptyWorkflowTranscript } from './workflow-transcript.ts';
import type { TurnLive } from './delta.ts';
import type { ProjectedDeltaFrame, ProjectedEventData, ProjectedEventFrame, ProjectedStreamFrame } from './projected.ts';
import type { WorkflowTranscript } from './workflow-transcript.ts';

export type RunState = 'idle' | 'running' | 'failed' | 'canceled';
export type ToolRequestStatus = 'pending' | 'resolved' | 'closed';
export type WorkflowStatus = 'running' | 'completed' | 'failed' | 'canceled';

export type TranscriptMessage =
  | { role: 'user'; key: string; text: string; at: number; runSeq: number | null }
  | { role: 'assistant'; key: string; text: string | null; toolCalls: { id: string; name: string }[]; at: number; streamed: boolean }
  | { role: 'tool'; key: string; name: string; phase: 'executed' | 'rejected'; ok: boolean | null; at: number }
  | {
      role: 'tool-request';
      key: string;
      seq: number;
      tool: string;
      toolCallId: string;
      agentId: string;
      workflowId?: string;
      data: unknown;
      status: ToolRequestStatus;
      result?: unknown;
      resolvedBy?: 'user' | 'server';
      settledAt?: number;
      at: number;
    }
  | {
      role: 'workflow';
      key: string;
      workflowId: string;
      app: string;
      name: string;
      status: WorkflowStatus;
      startedAt: number;
      finishedAt?: number;
      cursor: number;
      invocation?: string;
      transcript: WorkflowTranscript;
    }
  | { role: 'run-failed'; key: string; at: number };

export type ToolRequestMessage = Extract<TranscriptMessage, { role: 'tool-request' }>;
export type WorkflowMessage = Extract<TranscriptMessage, { role: 'workflow' }>;

type ToolRequestedData = Extract<ProjectedEventData, { kind: 'tool.requested' }>['data'];
type ToolResolvedData = Extract<ProjectedEventData, { kind: 'tool.resolved' }>['data'];

export type Transcript = {
  messages: TranscriptMessage[];
  run: RunState;
  turn: 'idle' | 'active';
  retrying: boolean;
  live: TurnLive | null;
  cursor: number;
  agentId: string | null;
  title: string | null;
};

export const emptyTranscript = (): Transcript => ({
  messages: [],
  run: 'idle',
  turn: 'idle',
  retrying: false,
  live: null,
  cursor: 0,
  agentId: null,
  title: null,
});

export const pendingRootRequests = (t: Transcript): ToolRequestMessage[] =>
  t.messages.filter((m): m is ToolRequestMessage => m.role === 'tool-request' && m.workflowId === undefined && m.status === 'pending');

export const runningWorkflows = (t: Transcript): WorkflowMessage[] =>
  t.messages.filter((m): m is WorkflowMessage => m.role === 'workflow' && m.status === 'running');

const toolMessage = (event: ProjectedEventFrame, name: string, phase: 'executed' | 'rejected', ok: boolean | null): TranscriptMessage => ({
  role: 'tool',
  key: `e${event.seq}`,
  name,
  phase,
  ok,
  at: event.occurredAt,
});

const requestMessage = (
  event: ProjectedEventFrame,
  data: ToolRequestedData,
  rootAgentId: string | null,
  workflowId?: string,
): TranscriptMessage => ({
  role: 'tool-request',
  key: workflowId === undefined ? `e${event.seq}` : `w${workflowId}:${event.seq}`,
  seq: event.seq,
  tool: data.tool,
  toolCallId: event.context.toolCallId ?? '',
  agentId: event.context.agent?.id ?? rootAgentId ?? '',
  workflowId,
  data: data.data,
  status: 'pending',
  at: event.occurredAt,
});

const resolveRequest = (
  messages: TranscriptMessage[],
  toolCallId: string | undefined,
  data: ToolResolvedData,
  at: number,
  workflowId?: string,
): TranscriptMessage[] =>
  messages.map((m) =>
    m.role === 'tool-request' && m.status === 'pending' && m.workflowId === workflowId && m.toolCallId === toolCallId
      ? {
          ...m,
          status: 'resolved' as const,
          result: data.data,
          ...(data.resolvedBy !== undefined && { resolvedBy: data.resolvedBy }),
          settledAt: at,
        }
      : m,
  );

const closePendingRequests = (messages: TranscriptMessage[], at: number, workflowId?: string): TranscriptMessage[] =>
  messages.map((m) =>
    m.role === 'tool-request' && m.status === 'pending' && m.workflowId === workflowId
      ? { ...m, status: 'closed' as const, settledAt: at }
      : m,
  );

const setWorkflowStatus = (messages: TranscriptMessage[], workflowId: string, status: WorkflowStatus): TranscriptMessage[] =>
  messages.map((m) => (m.role === 'workflow' && m.workflowId === workflowId ? { ...m, status } : m));

const settleWorkflow = (t: Transcript, workflowId: string, status: WorkflowStatus, at: number): Transcript => ({
  ...t,
  messages: setWorkflowStatus(closePendingRequests(t.messages, at, workflowId), workflowId, status).map((m) =>
    m.role === 'workflow' && m.workflowId === workflowId ? { ...m, finishedAt: at, transcript: { ...m.transcript, live: null } } : m,
  ),
});

const closeInvocationWorkflow = (t: Transcript, invocation: string | undefined, status: WorkflowStatus, at: number): Transcript => {
  const running = t.messages.filter((m): m is WorkflowMessage => m.role === 'workflow' && m.status === 'running');
  const last = running.at(-1);
  if (last === undefined) return t;

  const matched = invocation === undefined ? undefined : running.find((workflow) => workflow.invocation === invocation);
  if (matched !== undefined) return settleWorkflow(t, matched.workflowId, status, at);
  if (invocation !== undefined && running.some((workflow) => workflow.invocation !== undefined)) return t;

  return settleWorkflow(t, last.workflowId, status, at);
};

const applyWorkflowEvent = (t: Transcript, event: ProjectedEventFrame, workflowId: string): Transcript => {
  const workflow = t.messages.find((m): m is WorkflowMessage => m.role === 'workflow' && m.workflowId === workflowId);
  if (workflow === undefined || event.seq <= workflow.cursor) return t;

  const advance = (next: Transcript): Transcript => ({
    ...next,
    messages: next.messages.map((m) => (m.role === 'workflow' && m.workflowId === workflowId ? { ...m, cursor: event.seq } : m)),
  });

  return advance(
    match(event)
      .with({ kind: 'workflow.completed' }, () => settleWorkflow(t, workflowId, 'completed', event.occurredAt))
      .with({ kind: 'workflow.failed' }, () => settleWorkflow(t, workflowId, 'failed', event.occurredAt))
      .with({ kind: 'workflow.canceled' }, () => settleWorkflow(t, workflowId, 'canceled', event.occurredAt))
      .with({ kind: P.union('step.started', 'step.completed', 'turn.completed', 'tool.executed') }, () => ({
        ...t,
        messages: t.messages.map((m) =>
          m.role === 'workflow' && m.workflowId === workflowId
            ? { ...m, transcript: applyWorkflowTranscriptEvent(m.transcript, event) }
            : m,
        ),
      }))
      .with({ kind: 'tool.requested' }, ({ data }) => ({
        ...t,
        messages: [...t.messages, requestMessage(event, data, t.agentId, workflowId)],
      }))
      .with({ kind: 'tool.resolved' }, ({ data }) => ({
        ...t,
        messages: resolveRequest(t.messages, event.context.toolCallId, data, event.occurredAt, workflowId),
      }))
      .otherwise(() => t),
  );
};

const applyEvent = (t: Transcript, event: ProjectedEventFrame): Transcript => {
  if (event.source === 'WORKFLOW') return event.workflowId === undefined ? t : applyWorkflowEvent(t, event, event.workflowId);
  if (event.seq <= t.cursor) return t;

  const next = { ...t, cursor: event.seq, agentId: t.agentId ?? event.context.agent?.id ?? null };

  return match(event)
    .with({ kind: 'agent.created' }, () => next)
    .with({ kind: 'run.started' }, ({ data }) => {
      const text = data.command ? `/${data.command.name}${data.command.args === '' ? '' : ` ${data.command.args}`}` : data.message;
      const message: TranscriptMessage = {
        role: 'user',
        key: `e${event.seq}`,
        text,
        at: event.occurredAt,
        runSeq: event.context.run ?? null,
      };
      return { ...next, run: 'running' as const, retrying: false, messages: [...t.messages, message] };
    })
    .with({ kind: 'run.completed' }, () => ({
      ...next,
      run: 'idle' as const,
      turn: 'idle' as const,
      live: null,
      retrying: false,
      messages: closePendingRequests(t.messages, event.occurredAt),
    }))
    .with({ kind: 'run.failed' }, () => ({
      ...next,
      run: 'failed' as const,
      turn: 'idle' as const,
      live: null,
      retrying: false,
      messages: [
        ...closePendingRequests(t.messages, event.occurredAt),
        { role: 'run-failed' as const, key: `e${event.seq}`, at: event.occurredAt },
      ],
    }))
    .with({ kind: 'run.canceled' }, () => ({
      ...next,
      run: 'canceled' as const,
      turn: 'idle' as const,
      live: null,
      retrying: false,
      messages: closePendingRequests(t.messages, event.occurredAt),
    }))
    .with({ kind: 'turn.started' }, () => ({ ...next, turn: 'active' as const, live: startTurn(t.live, event.context), retrying: false }))
    .with({ kind: 'turn.retried' }, () => ({ ...next, turn: 'active' as const, retrying: true, live: null }))
    .with({ kind: 'turn.completed' }, ({ data }) => {
      const toolCalls = ('toolCalls' in data ? data.toolCalls : []).map((call) => ({ id: call.id, name: call.name }));
      const live = t.live;
      const streamed = live !== null && !live.seeded && live.text.length > 0 && sealTurn(live, event.context) === null;
      const sealed = { ...next, turn: 'idle' as const, live: sealTurn(t.live, event.context), retrying: false };
      if (data.text === null && toolCalls.length === 0) return sealed;
      const message: TranscriptMessage = {
        role: 'assistant',
        key: `e${event.seq}`,
        text: data.text,
        toolCalls,
        at: event.occurredAt,
        streamed,
      };
      return { ...sealed, messages: [...t.messages, message] };
    })
    .with({ kind: 'tool.executed' }, ({ data }) => ({
      ...next,
      messages: [...t.messages, toolMessage(event, data.tool, 'executed', data.ok)],
    }))
    .with({ kind: 'tool.rejected' }, ({ data }) => ({
      ...next,
      messages: [...t.messages, toolMessage(event, data.tool, 'rejected', false)],
    }))
    .with({ kind: 'tool.requested' }, ({ data }) => ({ ...next, messages: [...t.messages, requestMessage(event, data, next.agentId)] }))
    .with({ kind: 'tool.resolved' }, ({ data }) => ({
      ...next,
      messages: resolveRequest(t.messages, event.context.toolCallId, data, event.occurredAt),
    }))
    .with({ kind: 'invocation.started' }, ({ data }) =>
      match(data.target)
        .with({ kind: 'workflow' }, (target) => {
          const message: TranscriptMessage = {
            role: 'workflow',
            key: `e${event.seq}`,
            workflowId: target.id,
            app: target.app,
            name: target.name,
            status: 'running',
            startedAt: event.occurredAt,
            cursor: 0,
            invocation: event.context.invocation,
            transcript: emptyWorkflowTranscript(),
          };
          return { ...next, messages: [...t.messages, message] };
        })
        .with({ kind: 'agent' }, () => next)
        .exhaustive(),
    )
    .with({ kind: 'invocation.completed' }, () => closeInvocationWorkflow(next, event.context.invocation, 'completed', event.occurredAt))
    .with({ kind: 'invocation.failed' }, () => closeInvocationWorkflow(next, event.context.invocation, 'failed', event.occurredAt))
    .with({ kind: 'invocation.canceled' }, () => closeInvocationWorkflow(next, event.context.invocation, 'canceled', event.occurredAt))
    .with({ kind: 'assistant.titled' }, ({ data }) => ({ ...next, title: data.title }))
    .with(
      {
        kind: P.union('workflow.started', 'workflow.completed', 'workflow.failed', 'workflow.canceled', 'step.started', 'step.completed'),
      },
      () => t,
    )
    .exhaustive();
};

const applyWorkflowDelta = (t: Transcript, delta: ProjectedDeltaFrame): Transcript => ({
  ...t,
  messages: t.messages.map((m) =>
    m.role === 'workflow' && m.workflowId === delta.workflowId
      ? { ...m, transcript: applyWorkflowTranscriptDelta(m.transcript, delta) }
      : m,
  ),
});

export const applyFrame = (t: Transcript, frame: ProjectedStreamFrame): Transcript =>
  match(frame)
    .with({ type: 'sync' }, ({ seq }) => ({ ...t, cursor: Math.max(t.cursor, seq) }))
    .with({ type: 'delta' }, ({ delta }) =>
      delta.workflowId === undefined ? { ...t, live: applyDelta(t.live, delta) } : applyWorkflowDelta(t, delta),
    )
    .with({ type: 'event' }, ({ event }) => applyEvent(t, event))
    .exhaustive();

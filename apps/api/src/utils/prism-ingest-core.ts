// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { AskQuestionsSchema, WorkflowUsageSchema } from '@typie/prism';
import { z } from 'zod';
import type { PrismRunState, PrismWorkflowState } from '@typie/lib/enums';
import type { AskQuestion, EventFrame, ParkedScope, ProjectedDeltaFrame, ProjectedStreamFrame, RunUsage, TurnLive } from '@typie/prism';

export type IngestTarget = { kind: 'agent'; sessionId: string } | { kind: 'workflow'; workflowId: string };

export const logKeyOf = (target: IngestTarget): string =>
  target.kind === 'agent' ? `agent-${target.sessionId}` : `workflow-${target.workflowId}`;

export const parseLogKey = (key: string): IngestTarget | null => {
  const at = key.indexOf('-');
  if (at === -1) return null;
  const kind = key.slice(0, at);
  const id = key.slice(at + 1);
  if (id.length === 0) return null;
  if (kind === 'agent') return { kind: 'agent', sessionId: id };
  if (kind === 'workflow') return { kind: 'workflow', workflowId: id };
  return null;
};

export const PARKED_KINDS: ReadonlySet<string> = new Set([
  'run.started',
  'run.completed',
  'run.failed',
  'run.canceled',
  'tool.requested',
  'tool.resolved',
  'invocation.started',
  'invocation.completed',
  'invocation.failed',
  'invocation.canceled',
]);

const RUN_TERMINAL: Record<string, PrismRunState | undefined> = {
  'run.completed': 'COMPLETED',
  'run.failed': 'FAILED',
  'run.canceled': 'CANCELED',
};
const WORKFLOW_TERMINAL: Record<string, PrismWorkflowState | undefined> = {
  'workflow.completed': 'COMPLETED',
  'workflow.failed': 'FAILED',
  'workflow.canceled': 'CANCELED',
};

export type DomainOp =
  | { op: 'run-started'; runSeq: number; at: number }
  | { op: 'run-terminal'; runSeq: number; state: PrismRunState; at: number }
  | { op: 'workflow-link'; descriptor: { prismWorkflowId: string; app: string; name: string; ref: string | null; startedAt: number } }
  | { op: 'titled'; title: string }
  | { op: 'ask-push'; toolCallId: string; questions: AskQuestion[]; at: number }
  | { op: 'workflow-settle'; state: PrismWorkflowState; result: unknown; usage: RunUsage | null; error: string | null; at: number };

export type EventPlan = { advance: boolean; ops: DomainOp[]; sealTurn: boolean; clearLive: boolean };

const WorkflowTargetSchema = z.object({
  kind: z.literal('workflow'),
  id: z.string(),
  name: z.string(),
  app: z.string(),
  ref: z.string().nullable().optional(),
});

const usageOf = (raw: unknown): RunUsage | null => {
  const parsed = WorkflowUsageSchema.nullable().safeParse(raw ?? null);
  if (!parsed.success || parsed.data === null) return null;
  return { complete: parsed.data.settled ? parsed.data.complete : false, folds: parsed.data.folds };
};

const askPush = (event: EventFrame): DomainOp[] => {
  const toolCallId = event.context?.toolCallId;
  if (toolCallId === undefined || event.data.tool !== 'ask-user') return [];
  const parsed = AskQuestionsSchema.safeParse(event.data.data);
  return parsed.success ? [{ op: 'ask-push', toolCallId, questions: parsed.data.questions, at: event.occurredAt }] : [];
};

const agentOps = (event: EventFrame): DomainOp[] => {
  const context = event.context;
  if (context === null) return [];

  if (event.kind === 'run.started' && typeof context.run === 'number')
    return [{ op: 'run-started', runSeq: context.run, at: event.occurredAt }];
  const terminal = RUN_TERMINAL[event.kind];
  if (terminal !== undefined && typeof context.run === 'number') {
    return [{ op: 'run-terminal', runSeq: context.run, state: terminal, at: event.occurredAt }];
  }
  if (event.kind === 'invocation.started') {
    const target = WorkflowTargetSchema.safeParse(event.data.target);
    if (!target.success) return [];
    const { id, app, name, ref } = target.data;
    return [{ op: 'workflow-link', descriptor: { prismWorkflowId: id, app, name, ref: ref ?? null, startedAt: event.occurredAt } }];
  }
  if (event.kind === 'assistant.titled' && typeof event.data.title === 'string') return [{ op: 'titled', title: event.data.title }];
  if (event.kind === 'tool.requested') return askPush(event);
  return [];
};

const workflowOps = (event: EventFrame): DomainOp[] => {
  if (event.context === null) return [];

  const settled = WORKFLOW_TERMINAL[event.kind];
  if (settled !== undefined) {
    return [
      {
        op: 'workflow-settle',
        state: settled,
        result: event.data.result ?? null,
        usage: usageOf(event.data.usage),
        error: typeof event.data.reason === 'string' ? event.data.reason : null,
        at: event.occurredAt,
      },
    ];
  }
  if (event.kind === 'tool.requested') return askPush(event);
  return [];
};

export const planEvent = (scope: ParkedScope, event: EventFrame, cursor: number): EventPlan => ({
  advance: event.seq > cursor,
  ops: scope === 'agent' ? agentOps(event) : workflowOps(event),
  sealTurn: event.context !== null && event.kind === 'turn.completed',
  clearLive: event.context !== null && (scope === 'agent' ? RUN_TERMINAL : WORKFLOW_TERMINAL)[event.kind] !== undefined,
});

export const shouldStop = (state: { synced: boolean; open: boolean; parked: boolean }): boolean =>
  state.synced && (!state.open || state.parked);

const ABSENT_BASE_MS = 1000;
const ABSENT_MAX_MS = 30_000;
const ABSENT_GIVE_UP_MS = 5 * 60_000;

export const absentDelay = (absent: number, elapsedMs: number): number | null =>
  elapsedMs > ABSENT_GIVE_UP_MS ? null : Math.min(ABSENT_BASE_MS * 2 ** (absent - 1), ABSENT_MAX_MS);

export type LiveScope = { source: 'SESSION' } | { source: 'WORKFLOW'; workflowId: string };

export const liveFieldKey = (scope: LiveScope, agentId: string): string =>
  scope.source === 'SESSION' ? `S|${agentId}` : `W|${scope.workflowId}|${agentId}`;

const scopeOfField = (field: string): { workflowId?: string } => {
  const [kind, second] = field.split('|');
  return kind === 'W' && second ? { workflowId: second } : {};
};

export const liveSnapshotFrames = (fields: Record<string, TurnLive>): ProjectedDeltaFrame[] => {
  const frames: ProjectedDeltaFrame[] = [];
  for (const [field, live] of Object.entries(fields)) {
    const scope = scopeOfField(field);
    const candidates: { channel: TurnLive['last']; frame: ProjectedDeltaFrame }[] = [];
    if (live.text.length > 0) {
      candidates.push({
        channel: 'text',
        frame: { context: live.context, channel: 'text', offset: 0, data: live.text, seed: true, ...scope },
      });
    }
    if (live.thinkingChars > 0) {
      candidates.push({
        channel: 'thinking',
        frame: { context: live.context, channel: 'thinking', chars: live.thinkingChars, seed: true, ...scope },
      });
    }
    if (live.toolInput !== null) {
      candidates.push({
        channel: 'tool.input',
        frame: { context: live.context, channel: 'tool.input', tool: { id: null, name: live.toolInput.name }, seed: true, ...scope },
      });
    }
    frames.push(
      ...candidates.filter((c) => c.channel !== live.last).map((c) => c.frame),
      ...candidates.filter((c) => c.channel === live.last).map((c) => c.frame),
    );
  }
  return frames;
};

export const createFrameGate = (cursor: number, workflows: Map<string, number>) => {
  let session = cursor;
  const seen = new Map(workflows);
  return {
    accept(frame: ProjectedStreamFrame): boolean {
      if (frame.type !== 'event') return true;
      const { event } = frame;
      if (event.source === 'SESSION') {
        if (event.seq <= session) return false;
        session = event.seq;
        return true;
      }
      const key = event.workflowId ?? '';
      if (event.seq <= (seen.get(key) ?? 0)) return false;
      seen.set(key, event.seq);
      return true;
    },
  };
};

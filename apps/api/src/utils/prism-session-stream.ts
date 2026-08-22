import type { StreamFrame } from '@typie/prism';

const RUN_TERMINAL = new Set(['run.completed', 'run.failed', 'run.canceled']);
const WORKFLOW_TERMINAL = new Set(['workflow.completed', 'workflow.failed', 'workflow.canceled']);

export type SessionRoute =
  | { kind: 'workflow-started'; workflowId: string; app: string; name: string; ref: string | null; startedAt: number }
  | { kind: 'invocation-retried' }
  | { kind: 'run-terminal'; runSeq: number }
  | { kind: 'workflow-terminal' }
  | { kind: 'titled'; title: string };

export const routeSessionFrame = (frame: StreamFrame): SessionRoute | null => {
  if (frame.type !== 'event') return null;

  const { kind, data, context, occurredAt } = frame.event;

  if (kind === 'invocation.started') {
    const target = data.target as { kind?: string; id?: string; app?: string; name?: string; ref?: unknown } | undefined;
    if (target?.kind !== 'workflow' || typeof target.id !== 'string' || typeof target.app !== 'string' || typeof target.name !== 'string') {
      return null;
    }

    return {
      kind: 'workflow-started',
      workflowId: target.id,
      app: target.app,
      name: target.name,
      ref: typeof target.ref === 'string' ? target.ref : null,
      startedAt: occurredAt,
    };
  }

  if (kind === 'invocation.retried') return { kind: 'invocation-retried' };

  if (RUN_TERMINAL.has(kind)) return typeof context?.run === 'number' ? { kind: 'run-terminal', runSeq: context.run } : null;
  if (WORKFLOW_TERMINAL.has(kind)) return { kind: 'workflow-terminal' };

  if (kind === 'assistant.titled') return typeof data.title === 'string' ? { kind: 'titled', title: data.title } : null;

  return null;
};

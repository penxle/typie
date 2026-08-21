import { ASK_USER_TOOL, ProjectedEventSchema, ProjectedWorkflowEventSchema } from '@typie/prism';
import { match } from 'ts-pattern';
import type { DeltaFrame, ProjectedDeltaFrame, ProjectedStreamFrame, StreamFrame } from '@typie/prism';
import type { ZodError } from 'zod';

export type ProjectedScope = { source: 'SESSION' } | { source: 'WORKFLOW'; workflowId: string };

const isUnknownKind = (error: ZodError): boolean =>
  error.issues.length === 1 && error.issues[0].code === 'invalid_union' && error.issues[0].path[0] === 'kind';

const projectDelta = (delta: DeltaFrame): ProjectedDeltaFrame =>
  match(delta)
    .with({ channel: 'text' }, ({ context, offset, data }) => ({ context, channel: 'text', offset, data }) as const)
    .with({ channel: 'thinking' }, ({ context, offset, data }) => ({ context, channel: 'thinking', chars: offset + data.length }) as const)
    .with({ channel: 'tool.input' }, ({ context, tool }) => ({ context, channel: 'tool.input', tool }) as const)
    .exhaustive();

const WORKFLOW_TOOL_KINDS = new Set(['tool.requested', 'tool.resolved']);

const WORKFLOW_TOOLS = new Set<unknown>([ASK_USER_TOOL]);

export const projectFrame = (frame: StreamFrame, scope: ProjectedScope): ProjectedStreamFrame | null =>
  match(frame)
    .with({ type: 'heartbeat' }, () => null)
    .with({ type: 'sync' }, ({ seq }) => (scope.source === 'SESSION' ? ({ type: 'sync', seq } as const) : null))
    .with({ type: 'delta' }, ({ delta }) => ({
      type: 'delta' as const,
      delta: { ...projectDelta(delta), ...(scope.source === 'WORKFLOW' && { workflowId: scope.workflowId }) },
    }))
    .with({ type: 'event' }, ({ event }) => {
      if (scope.source === 'WORKFLOW' && WORKFLOW_TOOL_KINDS.has(event.kind) && !WORKFLOW_TOOLS.has(event.data.tool)) {
        return null;
      }

      const schema = scope.source === 'SESSION' ? ProjectedEventSchema : ProjectedWorkflowEventSchema;
      const parsed = schema.safeParse({ kind: event.kind, data: event.data });
      if (!parsed.success) {
        if (isUnknownKind(parsed.error)) {
          return null;
        }

        throw parsed.error;
      }

      if (event.context === null) {
        throw new Error(`event ${event.seq} (${event.kind}) has no context`);
      }

      return {
        type: 'event' as const,
        event: {
          seq: event.seq,
          occurredAt: event.occurredAt,
          context: event.context,
          source: scope.source,
          ...(scope.source === 'WORKFLOW' && { workflowId: scope.workflowId }),
          ...parsed.data,
        },
      };
    })
    .exhaustive();

import { ProjectedEventSchema } from '@typie/prism';
import { match } from 'ts-pattern';
import type { DeltaFrame, ProjectedDeltaFrame, ProjectedStreamFrame, StreamFrame } from '@typie/prism';
import type { ZodError } from 'zod';

const isUnknownKind = (error: ZodError): boolean =>
  error.issues.length === 1 && error.issues[0].code === 'invalid_union' && error.issues[0].path[0] === 'kind';

const projectDelta = (delta: DeltaFrame): ProjectedDeltaFrame =>
  match(delta)
    .with({ channel: 'text' }, ({ context, offset, data }) => ({ context, channel: 'text', offset, data }) as const)
    .with({ channel: 'thinking' }, ({ context, offset, data }) => ({ context, channel: 'thinking', chars: offset + data.length }) as const)
    .with({ channel: 'tool.input' }, ({ context, tool }) => ({ context, channel: 'tool.input', tool }) as const)
    .exhaustive();

export const projectFrame = (frame: StreamFrame): ProjectedStreamFrame | null =>
  match(frame)
    .with({ type: 'heartbeat' }, () => null)
    .with({ type: 'sync' }, ({ seq }) => ({ type: 'sync', seq }) as const)
    .with({ type: 'delta' }, ({ delta }) => ({ type: 'delta', delta: projectDelta(delta) }) as const)
    .with({ type: 'event' }, ({ event }) => {
      const parsed = ProjectedEventSchema.safeParse({ kind: event.kind, data: event.data });
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
          ...parsed.data,
        },
      };
    })
    .exhaustive();

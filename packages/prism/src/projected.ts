import { z } from 'zod';
import type { Context, TurnContext } from './wire.ts';

const EmptyData = z.object({});
const ToolResultData = z.object({ tool: z.string(), ok: z.boolean() });
const ToolCallData = z.object({ tool: z.string() });
const TurnCompletedData = z.object({
  text: z.string().nullable(),
  toolCalls: z.array(
    z.union([
      z.object({ kind: z.literal('parsed'), id: z.string(), name: z.string(), input: z.unknown() }),
      z.object({ kind: z.literal('malformed'), id: z.string(), name: z.string() }),
    ]),
  ),
});

export const ProjectedEventSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('run.started'), data: z.object({ message: z.string() }) }),
  z.object({ kind: z.literal('run.completed'), data: EmptyData }),
  z.object({ kind: z.literal('run.failed'), data: EmptyData }),
  z.object({ kind: z.literal('run.canceled'), data: EmptyData }),
  z.object({ kind: z.literal('turn.started'), data: EmptyData }),
  z.object({ kind: z.literal('turn.retried'), data: EmptyData }),
  z.object({ kind: z.literal('turn.completed'), data: TurnCompletedData }),
  z.object({ kind: z.literal('tool.executed'), data: ToolResultData }),
  z.object({ kind: z.literal('tool.resolved'), data: ToolResultData }),
  z.object({ kind: z.literal('tool.rejected'), data: ToolCallData }),
  z.object({ kind: z.literal('tool.requested'), data: ToolCallData }),
]);
export type ProjectedEventData = z.infer<typeof ProjectedEventSchema>;

export type ProjectedDeltaFrame =
  | { context: TurnContext; channel: 'text'; offset: number; data: string }
  | { context: TurnContext; channel: 'thinking'; chars: number }
  | { context: TurnContext; channel: 'tool.input'; tool: { id: string | null; name: string } };

export type ProjectedEventFrame = ProjectedEventData & {
  seq: number;
  occurredAt: number;
  context: Context;
};

export type ProjectedStreamFrame =
  { type: 'event'; event: ProjectedEventFrame } | { type: 'delta'; delta: ProjectedDeltaFrame } | { type: 'sync'; seq: number };

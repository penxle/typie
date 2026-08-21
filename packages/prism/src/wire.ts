import { z } from 'zod';

export const AgentRefSchema = z.object({ id: z.string(), name: z.string() });
export type AgentRef = z.infer<typeof AgentRefSchema>;

export const ContextSchema = z.object({
  step: z.string().optional(),
  invocation: z.string().optional(),
  agent: AgentRefSchema.optional(),
  run: z.number().optional(),
  turn: z.number().optional(),
  attempt: z.number().optional(),
  toolCallId: z.string().optional(),
});
export type Context = z.infer<typeof ContextSchema>;

export const TurnContextSchema = z.object({
  agent: AgentRefSchema,
  run: z.number(),
  turn: z.number(),
  attempt: z.number(),
  invocation: z.string().optional(),
  step: z.string().optional(),
});
export type TurnContext = z.infer<typeof TurnContextSchema>;

export const EventFrameSchema = z.object({
  seq: z.number(),
  kind: z.string(),
  occurredAt: z.number(),
  context: ContextSchema.nullable(),
  data: z.record(z.string(), z.unknown()),
});
export type EventFrame = z.infer<typeof EventFrameSchema>;

export const DeltaFrameSchema = z.union([
  z.object({ context: TurnContextSchema, channel: z.enum(['text', 'thinking']), offset: z.number(), data: z.string() }),
  z.object({
    context: TurnContextSchema,
    channel: z.literal('tool.input'),
    offset: z.number(),
    data: z.string(),
    tool: z.object({ id: z.string().nullable(), name: z.string() }),
  }),
]);
export type DeltaFrame = z.infer<typeof DeltaFrameSchema>;

export const SyncDataSchema = z.object({ seq: z.number() });

const JsonText = z.string().transform((text, ctx): unknown => {
  try {
    return JSON.parse(text);
  } catch {
    ctx.addIssue({ code: 'custom', message: 'invalid json' });
    return z.NEVER;
  }
});

export const StreamFrameSchema = z.union([
  z.object({ event: z.literal('heartbeat') }).transform(() => ({ type: 'heartbeat' }) as const),
  z
    .object({ event: z.literal('sync'), data: JsonText.pipe(SyncDataSchema) })
    .transform(({ data }) => ({ type: 'sync', seq: data.seq }) as const),
  z
    .object({ event: z.literal('turn.delta'), data: JsonText.pipe(DeltaFrameSchema) })
    .transform(({ data }) => ({ type: 'delta', delta: data }) as const),
  z.object({ event: z.string(), data: JsonText.pipe(EventFrameSchema) }).transform(({ data }) => ({ type: 'event', event: data }) as const),
]);
export type StreamFrame = z.infer<typeof StreamFrameSchema>;

export const RunSummarySchema = z.object({ runSeq: z.number(), status: z.string() });
export type RunSummary = z.infer<typeof RunSummarySchema>;

export const InvocationSummarySchema = z.object({
  invocationId: z.string(),
  targetKind: z.string(),
  targetId: z.string(),
  originRunSeq: z.number().nullable(),
  status: z.string(),
});
export type InvocationSummary = z.infer<typeof InvocationSummarySchema>;

export const UsageFoldSchema = z.object({
  provider: z.string(),
  agent: z.string(),
  model: z.string(),
  effort: z.string().nullable(),
  turns: z.number(),
  inputTokens: z.number(),
  outputTokens: z.number(),
  cacheReadTokens: z.number(),
  cacheWriteTokens: z.number(),
  thinkingTokens: z.number().nullable(),
});
export type UsageFold = z.infer<typeof UsageFoldSchema>;

export type RunUsage = { complete: boolean; folds: UsageFold[] };

export const WorkflowUsageSchema = z.union([
  z.object({ settled: z.literal(true), complete: z.boolean(), folds: z.array(UsageFoldSchema) }),
  z.object({ settled: z.literal(false), folds: z.array(UsageFoldSchema) }),
]);

export const WorkflowStateSchema = z.object({
  workflow: z.object({
    id: z.string(),
    app: z.string().nullable(),
    workflow: z.string().nullable(),
    ref: z.string().nullable(),
    status: z.string(),
    result: z.string().nullable(),
    error: z.string().nullable(),
    usage: WorkflowUsageSchema.nullable(),
    startedAt: z.number(),
    finishedAt: z.number().nullable(),
  }),
  invocations: z.array(InvocationSummarySchema),
});
export type WorkflowState = z.infer<typeof WorkflowStateSchema>;

export const AgentStateSchema = z.object({
  runs: z.array(RunSummarySchema),
  pending: z.object({ toolCallId: z.string(), tool: z.string(), input: z.unknown(), data: z.unknown() }).nullable(),
  invocations: z.array(InvocationSummarySchema),
});
export type AgentState = z.infer<typeof AgentStateSchema>;

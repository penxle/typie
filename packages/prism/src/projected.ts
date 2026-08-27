import { z } from 'zod';
import { ConfirmDecisionSchema, ConfirmHintSchema } from './review.ts';
import { AskAnswersSchema, AskQuestionsSchema, ToolFailureSchema } from './tools.ts';
import type { Context, TurnContext } from './wire.ts';

const EmptyData = z.object({});

const DeleteTargetsData = z.object({ ids: z.array(z.string()) }).catch({ ids: [] });
const DeleteNoteTargetsData = z.object({ noteIds: z.array(z.string()) }).catch({ noteIds: [] });
const DeleteGoalTargetsData = z.object({ items: z.array(z.object({ id: z.string().optional() })) }).catch({ items: [] });
const SharingTargetsData = z
  .object({ ids: z.array(z.string()), visibility: z.enum(['PUBLIC', 'UNLISTED', 'PRIVATE']).nullable(), recursive: z.boolean().optional() })
  .catch({ ids: [], visibility: null });

const CountResultData = z.union([z.object({ ok: z.literal(true), count: z.number().int().nonnegative() }), ToolFailureSchema]);
const VISIBILITY = z.enum(['PUBLIC', 'UNLISTED', 'PRIVATE']);
const SharingChange = z.union([
  z.object({
    kind: z.literal('document'),
    documentId: z.string(),
    entityId: z.string(),
    title: z.string().nullable(),
    subtitle: z.string().nullable(),
    icon: z.string(),
    iconColor: z.string(),
    from: VISIBILITY,
    to: VISIBILITY,
  }),
  z.object({
    kind: z.literal('folder'),
    folderId: z.string(),
    entityId: z.string(),
    name: z.string(),
    icon: z.string(),
    iconColor: z.string(),
    from: VISIBILITY,
    to: VISIBILITY,
  }),
]);
const SharingResultData = z.union([
  z.object({ ok: z.literal(true), count: z.number().int().nonnegative(), changes: z.array(SharingChange) }),
  ToolFailureSchema,
]);
const SaveDocumentRequestData = z.object({ path: z.string(), summary: z.string() }).catch({ path: '', summary: '' });
const DocumentRefData = z.object({
  kind: z.literal('document'),
  documentId: z.string(),
  entityId: z.string(),
  title: z.string().nullable(),
  subtitle: z.string().nullable(),
  icon: z.string(),
  iconColor: z.string(),
});
const ChangedCountsData = z.object({
  blocks: z.object({
    inserted: z.number().int().nonnegative(),
    deleted: z.number().int().nonnegative(),
    moved: z.number().int().nonnegative(),
    updated: z.number().int().nonnegative(),
  }),
  chars: z.object({ inserted: z.number().int().nonnegative(), deleted: z.number().int().nonnegative() }),
});
const OpenDocumentResultData = z.union([z.object({ ok: z.literal(true), document: DocumentRefData, path: z.string() }), ToolFailureSchema]);
const SaveDocumentResultData = z.union([
  z.object({
    ok: z.literal(true),
    unchanged: z.boolean(),
    document: DocumentRefData,
    path: z.string(),
    changed: ChangedCountsData,
  }),
  ToolFailureSchema,
]);

const TOOL_REQUEST_DATA: Record<string, z.ZodType | undefined> = {
  'list-open-documents': EmptyData,
  'confirm-review': ConfirmHintSchema,
  'ask-user': AskQuestionsSchema,
  'delete-entities': DeleteTargetsData,
  'delete-notes': DeleteNoteTargetsData,
  'delete-goals': DeleteGoalTargetsData,
  'update-sharing': SharingTargetsData,
  'save-document': SaveDocumentRequestData,
};

const TOOL_RESULT_DATA: Record<string, z.ZodType | undefined> = {
  'confirm-review': ConfirmDecisionSchema,
  'ask-user': AskAnswersSchema,
  'delete-entities': CountResultData,
  'delete-notes': CountResultData,
  'delete-goals': CountResultData,
  'update-sharing': SharingResultData,
  'open-document': OpenDocumentResultData,
  'save-document': SaveDocumentResultData,
};

const narrowData = (
  registry: Record<string, z.ZodType | undefined>,
  tool: string,
  data: unknown,
  ctx: z.RefinementCtx,
): { data?: unknown } | null => {
  const schema = registry[tool];
  if (!schema) {
    return {};
  }

  const result = schema.safeParse(data);
  if (!result.success) {
    for (const issue of result.error.issues) {
      ctx.addIssue({ code: 'custom', message: issue.message, path: ['data', ...issue.path] });
    }

    return null;
  }

  return { data: result.data };
};

const RunStartedData = z
  .object({ message: z.string(), command: z.object({ name: z.string(), args: z.string() }).nullable().optional() })
  .transform(({ message, command }) => ({ message, command: command ?? null }));
const ToolCallData = z.object({ tool: z.string() });
const ToolOutcomeData = z.object({ tool: z.string(), ok: z.boolean() });
const TurnCompletedData = z.object({
  text: z.string().nullable(),
  toolCalls: z.array(
    z.union([
      z.object({ kind: z.literal('parsed'), id: z.string(), name: z.string(), input: z.unknown() }),
      z.object({ kind: z.literal('malformed'), id: z.string(), name: z.string() }),
    ]),
  ),
});
const ToolRequestedData = z
  .object({ tool: z.string(), data: z.unknown() })
  .transform(({ tool, data }, ctx): { tool: string; data?: unknown } => {
    const narrowed = narrowData(TOOL_REQUEST_DATA, tool, data, ctx);
    return narrowed ? { tool, ...narrowed } : z.NEVER;
  });
const ToolResolvedData = z
  .object({ tool: z.string(), ok: z.boolean(), data: z.unknown(), resolvedBy: z.enum(['user', 'server']).optional() })
  .transform(({ tool, ok, data, resolvedBy }, ctx): { tool: string; ok: boolean; data?: unknown; resolvedBy?: 'user' | 'server' } => {
    const narrowed = ok ? narrowData(TOOL_RESULT_DATA, tool, data, ctx) : {};
    return narrowed ? { tool, ok, ...(resolvedBy !== undefined && { resolvedBy }), ...narrowed } : z.NEVER;
  });
const InvocationStartedData = z.object({
  target: z.union([
    z.object({ kind: z.literal('workflow'), id: z.string(), name: z.string(), app: z.string() }),
    z.object({ kind: z.literal('agent') }),
  ]),
});

const ToolInputSummary = z.object({ path: z.string().optional(), query: z.string().optional() }).catch({});
const WorkflowTurnCompletedData = z.object({ text: z.string().nullable() });
const WorkflowToolExecutedData = z.object({ tool: z.string(), ok: z.boolean(), input: ToolInputSummary });

export const ProjectedEventSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('agent.created'), data: EmptyData }),
  z.object({ kind: z.literal('run.started'), data: RunStartedData }),
  z.object({ kind: z.literal('run.completed'), data: EmptyData }),
  z.object({ kind: z.literal('run.failed'), data: EmptyData }),
  z.object({ kind: z.literal('run.canceled'), data: EmptyData }),
  z.object({ kind: z.literal('turn.started'), data: EmptyData }),
  z.object({ kind: z.literal('turn.retried'), data: EmptyData }),
  z.object({ kind: z.literal('turn.completed'), data: TurnCompletedData }),
  z.object({ kind: z.literal('tool.executed'), data: ToolOutcomeData }),
  z.object({ kind: z.literal('tool.resolved'), data: ToolResolvedData }),
  z.object({ kind: z.literal('tool.rejected'), data: ToolCallData }),
  z.object({ kind: z.literal('tool.requested'), data: ToolRequestedData }),
  z.object({ kind: z.literal('invocation.started'), data: InvocationStartedData }),
  z.object({ kind: z.literal('invocation.completed'), data: EmptyData }),
  z.object({ kind: z.literal('invocation.failed'), data: EmptyData }),
  z.object({ kind: z.literal('invocation.canceled'), data: EmptyData }),
  z.object({ kind: z.literal('assistant.titled'), data: z.object({ title: z.string() }) }),
]);

export const ProjectedWorkflowEventSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('workflow.started'), data: EmptyData }),
  z.object({ kind: z.literal('workflow.completed'), data: EmptyData }),
  z.object({ kind: z.literal('workflow.failed'), data: EmptyData }),
  z.object({ kind: z.literal('workflow.canceled'), data: EmptyData }),
  z.object({ kind: z.literal('step.started'), data: EmptyData }),
  z.object({ kind: z.literal('step.completed'), data: EmptyData }),
  z.object({ kind: z.literal('turn.completed'), data: WorkflowTurnCompletedData }),
  z.object({ kind: z.literal('tool.executed'), data: WorkflowToolExecutedData }),
  z.object({ kind: z.literal('tool.requested'), data: ToolRequestedData }),
  z.object({ kind: z.literal('tool.resolved'), data: ToolResolvedData }),
]);

export type ProjectedSource = 'SESSION' | 'WORKFLOW';
export type ProjectedEventData = z.infer<typeof ProjectedEventSchema> | z.infer<typeof ProjectedWorkflowEventSchema>;

type DeltaScope = { workflowId?: string; seed?: boolean };

export type ProjectedDeltaFrame =
  | ({ context: TurnContext; channel: 'text'; offset: number; data: string } & DeltaScope)
  | ({ context: TurnContext; channel: 'thinking'; chars: number } & DeltaScope)
  | ({ context: TurnContext; channel: 'tool.input'; tool: { id: string | null; name: string } } & DeltaScope);

export type ProjectedEventFrame = ProjectedEventData & {
  seq: number;
  occurredAt: number;
  context: Context;
  source: ProjectedSource;
  workflowId?: string;
};

export type ProjectedStreamFrame =
  { type: 'event'; event: ProjectedEventFrame } | { type: 'delta'; delta: ProjectedDeltaFrame } | { type: 'sync'; seq: number };

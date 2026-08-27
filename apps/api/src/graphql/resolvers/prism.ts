import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import {
  EntityAvailability,
  PrismReaction,
  PrismRunState,
  PrismToolPhase,
  PrismToolPolicy,
  PrismToolRequestStatus,
  PrismToolResolver,
  PrismTurnState,
  PrismWorkflowState,
} from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import { prismSchema } from '@typie/lib/validation';
import { ApproveInputSchema, DECLINED_MESSAGE, serveVerdict, toGraphQL, TOOL_META, toolFailure } from '@typie/prism';
import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, inArray, isNotNull, isNull, sql } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import { nanoid } from 'nanoid';
import { redis } from '#/cache.ts';
import { clearLoaders } from '#/context.ts';
import {
  db,
  Documents,
  Entities,
  first,
  firstOrThrow,
  Folders,
  Notes,
  PrismReviewRounds,
  PrismRuns,
  PrismSessionEvents,
  PrismSessions,
  PrismWorkflowEvents,
  PrismWorkflows,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { env, lane } from '#/env.ts';
import { activeRun, newAgentId, prism, PrismApiError } from '#/external/prism.ts';
import { ensureIngest } from '#/mq/prism-queue.ts';
import { pubsub } from '#/pubsub.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { assertPrismAccess } from '#/utils/prism-access.ts';
import { parseAllowlist } from '#/utils/prism-access-core.ts';
import { prismCommands } from '#/utils/prism-catalog.ts';
import { projectFrame } from '#/utils/prism-events.ts';
import { createFrameGate, liveFieldKey, liveSnapshotFrames } from '#/utils/prism-ingest-core.ts';
import { runSite } from '#/utils/prism-serve.ts';
import { recordToolResolution, withToolLedger } from '#/utils/prism-tool-calls.ts';
import { ERROR_MESSAGE } from '#/utils/prism-tool-messages.ts';
import { prismTools } from '#/utils/prism-tools.ts';
import { materialize } from '#/utils/prism-transcript.ts';
import { cancelActiveRun, cancelSessionWorkflows, closeRun } from '#/utils/prism-workflows.ts';
import { entityRefFilter } from '#/utils/prism-workspace.ts';
import { builder } from '../builder.ts';
import { Entity, Note, PrismSession, PrismWorkflow, User } from '../objects.ts';
import type {
  ProjectedDeltaFrame,
  ProjectedStreamFrame,
  RunItemsWire,
  RunItemWire,
  TranscriptWire,
  TurnContext,
  TurnLive,
  WorkflowTranscriptWire,
} from '@typie/prism';
import type { StoredEvent } from '#/utils/prism-transcript.ts';

const log = logger.getChild('prism');

const PrismWorkflowCursorInput = builder.inputType('PrismWorkflowCursorInput', {
  fields: (t) => ({
    workflowId: t.string(),
    cursor: t.int(),
  }),
});

const toTypieError = (err: unknown): TypieError => {
  if (err instanceof PrismApiError) {
    log.warn('prism-api rejected: {code} ({status})', { code: err.code, status: err.status });
    if (err.code === 'run-active') return new TypieError({ code: 'prism_run_active', status: 409 });
    if (err.code === 'no-pending-tool') return new TypieError({ code: 'prism_tool_settled', status: 409 });
    if (err.code === 'unknown-command') return new TypieError({ code: 'prism_unknown_command', status: 409 });
    if (err.status >= 500 || err.code === 'internal' || err.code === 'malformed-response')
      return new TypieError({ code: 'prism_unavailable', status: 502 });
    return new TypieError({ code: `prism_rejected:${err.code}`, status: err.status });
  }
  log.error('prism call failed {*}', { error: err });
  Sentry.captureException(err);
  return new TypieError({ code: 'prism_unavailable', status: 502 });
};

export const prismError = (err: unknown): never => {
  throw toTypieError(err);
};

export const ownedSession = async (sessionId: string, userId: string) => {
  const session = await db
    .select()
    .from(PrismSessions)
    .where(and(eq(PrismSessions.id, sessionId), eq(PrismSessions.userId, userId), isNull(PrismSessions.deletedAt)))
    .then(first);
  if (!session) throw new TypieError({ code: 'not_found', status: 404 });
  return session;
};

const PrismCommand = builder.simpleObject('PrismCommand', {
  fields: (t) => ({
    name: t.string(),
    description: t.string(),
    argumentHint: t.string({ nullable: true }),
  }),
});

type ItemOf<K extends RunItemWire['kind']> = Extract<RunItemWire, { kind: K }>;
type PrismRunShape = RunItemsWire & { row: typeof PrismRuns.$inferSelect };
type PrismTranscriptShape = Omit<TranscriptWire<PrismRunShape>, 'runs'> & { runs: PrismRunShape[] };

const at = (value: string) => dayjs(value);
const atOrNull = (value: string | null) => (value === null ? null : dayjs(value));

const PrismToolCallRef = builder.simpleObject('PrismToolCallRef', { fields: (t) => ({ id: t.string(), name: t.string() }) });

const PrismUserMessage = builder.objectRef<ItemOf<'user'>>('PrismUserMessage').implement({
  fields: (t) => ({
    key: t.exposeString('key'),
    text: t.exposeString('text'),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
  }),
});

const PrismAssistantMessage = builder.objectRef<ItemOf<'assistant'>>('PrismAssistantMessage').implement({
  fields: (t) => ({
    key: t.exposeString('key'),
    text: t.exposeString('text', { nullable: true }),
    toolCalls: t.field({ type: [PrismToolCallRef], resolve: (s) => s.toolCalls }),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
    streamed: t.exposeBoolean('streamed'),
  }),
});

const PrismToolCall = builder.objectRef<ItemOf<'tool'>>('PrismToolCall').implement({
  fields: (t) => ({
    key: t.exposeString('key'),
    name: t.exposeString('name'),
    phase: t.field({ type: PrismToolPhase, resolve: (s) => s.phase }),
    ok: t.exposeBoolean('ok', { nullable: true }),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
  }),
});

const PrismToolRequest = builder.objectRef<ItemOf<'toolRequest'>>('PrismToolRequest').implement({
  fields: (t) => ({
    key: t.exposeString('key'),
    seq: t.exposeInt('seq'),
    tool: t.exposeString('tool'),
    toolCallId: t.exposeString('toolCallId'),
    agentId: t.exposeString('agentId'),
    workflowId: t.exposeString('workflowId', { nullable: true }),
    data: t.field({ type: 'JSON', nullable: true, resolve: (s) => s.data }),
    status: t.field({ type: PrismToolRequestStatus, resolve: (s) => s.status }),
    result: t.field({ type: 'JSON', nullable: true, resolve: (s) => s.result }),
    settledAt: t.field({ type: 'DateTime', nullable: true, resolve: (s) => atOrNull(s.settledAt) }),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
  }),
});

const PrismTranscriptStep = builder.objectRef<WorkflowTranscriptWire['steps'][number]>('PrismTranscriptStep').implement({
  fields: (t) => ({
    name: t.exposeString('name'),
    seq: t.exposeInt('seq'),
    startedAt: t.field({ type: 'DateTime', resolve: (s) => at(s.startedAt) }),
    completedAt: t.field({ type: 'DateTime', nullable: true, resolve: (s) => atOrNull(s.completedAt) }),
  }),
});

const PrismTranscriptTurn = builder.objectRef<WorkflowTranscriptWire['turns'][number]>('PrismTranscriptTurn').implement({
  fields: (t) => ({
    seq: t.exposeInt('seq'),
    step: t.exposeString('step', { nullable: true }),
    text: t.exposeString('text'),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
  }),
});

const PrismTranscriptTool = builder.objectRef<WorkflowTranscriptWire['tools'][number]>('PrismTranscriptTool').implement({
  fields: (t) => ({
    seq: t.exposeInt('seq'),
    step: t.exposeString('step', { nullable: true }),
    tool: t.exposeString('tool'),
    ok: t.exposeBoolean('ok'),
    path: t.exposeString('path', { nullable: true }),
    query: t.exposeString('query', { nullable: true }),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
  }),
});

const PrismWorkflowTranscript = builder.objectRef<WorkflowTranscriptWire>('PrismWorkflowTranscript').implement({
  fields: (t) => ({
    steps: t.field({ type: [PrismTranscriptStep], resolve: (s) => s.steps }),
    turns: t.field({ type: [PrismTranscriptTurn], resolve: (s) => s.turns }),
    tools: t.field({ type: [PrismTranscriptTool], resolve: (s) => s.tools }),
  }),
});

const PrismWorkflowRef = builder.objectRef<ItemOf<'workflow'>>('PrismWorkflowRef').implement({
  fields: (t) => ({
    key: t.exposeString('key'),
    prismWorkflowId: t.exposeString('prismWorkflowId'),
    app: t.exposeString('app'),
    name: t.exposeString('name'),
    status: t.field({ type: PrismWorkflowState, resolve: (s) => s.status }),
    startedAt: t.field({ type: 'DateTime', resolve: (s) => at(s.startedAt) }),
    finishedAt: t.field({ type: 'DateTime', nullable: true, resolve: (s) => atOrNull(s.finishedAt) }),
    cursor: t.exposeInt('cursor'),
    invocation: t.exposeString('invocation', { nullable: true }),
    transcript: t.field({ type: PrismWorkflowTranscript, resolve: (s) => s.transcript }),
    workflow: t.field({
      type: PrismWorkflow,
      nullable: true,
      resolve: (s) => db.select().from(PrismWorkflows).where(eq(PrismWorkflows.prismWorkflowId, s.prismWorkflowId)).then(first),
    }),
  }),
});

const PrismRunFailure = builder.objectRef<ItemOf<'runFailure'>>('PrismRunFailure').implement({
  fields: (t) => ({
    key: t.exposeString('key'),
    at: t.field({ type: 'DateTime', resolve: (s) => at(s.at) }),
  }),
});

const PrismRunItem = builder.unionType('PrismRunItem', {
  types: [PrismUserMessage, PrismAssistantMessage, PrismToolCall, PrismToolRequest, PrismWorkflowRef, PrismRunFailure],
  resolveType: (item: RunItemWire) =>
    ({
      user: PrismUserMessage,
      assistant: PrismAssistantMessage,
      tool: PrismToolCall,
      toolRequest: PrismToolRequest,
      workflow: PrismWorkflowRef,
      runFailure: PrismRunFailure,
    })[item.kind],
});

const PrismRun = builder.objectRef<PrismRunShape>('PrismRun').implement({
  fields: (t) => ({
    id: t.id({ resolve: (s) => s.row.id }),
    runSeq: t.int({ resolve: (s) => s.row.runSeq }),
    state: t.field({ type: PrismRunState, resolve: (s) => s.row.state }),
    startedAt: t.field({ type: 'DateTime', resolve: (s) => s.row.startedAt }),
    finishedAt: t.field({ type: 'DateTime', nullable: true, resolve: (s) => s.row.finishedAt }),
    reaction: t.field({ type: PrismReaction, nullable: true, resolve: (s) => s.row.reaction }),
    reactionNote: t.string({ nullable: true, resolve: (s) => s.row.reactionNote }),
    items: t.field({ type: [PrismRunItem], resolve: (s) => s.items }),
  }),
});

const PrismTranscript = builder.objectRef<PrismTranscriptShape>('PrismTranscript').implement({
  fields: (t) => ({
    cursor: t.exposeInt('cursor'),
    title: t.exposeString('title', { nullable: true }),
    agentId: t.exposeString('agentId', { nullable: true }),
    turn: t.field({ type: PrismTurnState, resolve: (s) => s.turn }),
    retrying: t.exposeBoolean('retrying'),
    runs: t.field({ type: [PrismRun], resolve: (s) => s.runs }),
  }),
});

const storedEvents = <
  R extends { seq: number; kind: string; occurredAt: dayjs.Dayjs; loggedAt: dayjs.Dayjs; context: unknown; data: Record<string, unknown> },
>(
  rows: R[],
): StoredEvent[] =>
  rows.map((row) => ({
    seq: row.seq,
    kind: row.kind,
    occurredAt: row.occurredAt.valueOf(),
    loggedAt: row.loggedAt.valueOf(),
    context: (row.context ?? null) as StoredEvent['context'],
    data: row.data,
  }));

const loadTranscript = async ({ id: sessionId, cursor }: { id: string; cursor: number }): Promise<PrismTranscriptShape> => {
  const [events, workflows, runs] = await Promise.all([
    db.select().from(PrismSessionEvents).where(eq(PrismSessionEvents.sessionId, sessionId)).orderBy(asc(PrismSessionEvents.seq)),
    db.select().from(PrismWorkflows).where(eq(PrismWorkflows.sessionId, sessionId)),
    db.select().from(PrismRuns).where(eq(PrismRuns.sessionId, sessionId)),
  ]);

  const workflowEvents = new Map<string, StoredEvent[]>();
  if (workflows.length > 0) {
    const rows = await db
      .select()
      .from(PrismWorkflowEvents)
      .where(
        inArray(
          PrismWorkflowEvents.workflowId,
          workflows.map((workflow) => workflow.id),
        ),
      )
      .orderBy(asc(PrismWorkflowEvents.seq));
    const prismIdOf = new Map(workflows.map((workflow) => [workflow.id, workflow.prismWorkflowId]));
    for (const row of rows) {
      const prismWorkflowId = prismIdOf.get(row.workflowId);
      if (prismWorkflowId === undefined) continue;
      const bucket = workflowEvents.get(prismWorkflowId) ?? [];
      bucket.push(...storedEvents([row]));
      workflowEvents.set(prismWorkflowId, bucket);
    }
  }

  const wire = toGraphQL(materialize(storedEvents(events), workflowEvents));
  const rowOf = new Map(runs.map((run) => [run.runSeq, run]));
  const shaped: PrismRunShape[] = [];
  for (const run of wire.runs) {
    const row = run.runSeq === null ? undefined : rowOf.get(run.runSeq);
    if (row === undefined) {
      log.warn('prism run row missing for transcript: {sessionId} run {runSeq}', { sessionId, runSeq: run.runSeq });
      continue;
    }
    shaped.push({ ...run, row });
  }
  return { ...wire, cursor: Math.max(wire.cursor, cursor), runs: shaped };
};

PrismSession.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.exposeString('title', { nullable: true }),
    toolPolicy: t.expose('toolPolicy', { type: PrismToolPolicy }),
    archivedAt: t.expose('archivedAt', { type: 'DateTime', nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

    awaitingUser: t.boolean({
      resolve: async (self, _, ctx) => {
        if (self.awaitingUserAt !== null) return true;

        const loader = ctx.loader({
          name: 'PrismSession.awaitingWorkflow',
          nullable: true,
          load: async (sessionIds: string[]) => {
            return await db
              .selectDistinct({ sessionId: PrismWorkflows.sessionId })
              .from(PrismWorkflows)
              .where(
                and(
                  inArray(PrismWorkflows.sessionId, sessionIds),
                  eq(PrismWorkflows.state, 'RUNNING'),
                  isNotNull(PrismWorkflows.awaitingUserAt),
                ),
              );
          },
          key: (row) => row?.sessionId,
        });

        return (await loader.load(self.id)) !== null;
      },
    }),

    unseenReviewCount: t.int({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'PrismSession.completedReviewRounds',
          many: true,
          load: async (sessionIds: string[]) => {
            return await db
              .select({ sessionId: PrismReviewRounds.sessionId, finishedAt: PrismWorkflows.finishedAt })
              .from(PrismReviewRounds)
              .innerJoin(PrismWorkflows, eq(PrismWorkflows.id, PrismReviewRounds.workflowId))
              .where(and(inArray(PrismReviewRounds.sessionId, sessionIds), eq(PrismWorkflows.state, 'COMPLETED')));
          },
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          key: ({ sessionId }) => sessionId!,
        });

        const rows = await loader.load(self.id);
        const seenAt = self.seenAt;
        if (seenAt === null) return rows.length;

        return rows.filter((row) => row.finishedAt !== null && row.finishedAt.isAfter(seenAt)).length;
      },
    }),

    transcript: t.field({ type: PrismTranscript, resolve: (self) => loadTranscript(self) }),
  }),
});

PrismWorkflow.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    prismWorkflowId: t.exposeString('prismWorkflowId'),
    app: t.exposeString('app'),
    name: t.exposeString('name'),
    state: t.expose('state', { type: PrismWorkflowState }),
    startedAt: t.expose('startedAt', { type: 'DateTime' }),
    finishedAt: t.expose('finishedAt', { type: 'DateTime', nullable: true }),
  }),
});

builder.objectFields(User, (t) => ({
  prismAccess: t.boolean({
    resolve: (self, _, ctx) => ctx.session?.userId === self.id && parseAllowlist(env.PRISM_BETA_USER_IDS).includes(self.id),
  }),

  prismCommands: t.field({
    type: [PrismCommand],
    nullable: true,
    resolve: async (self, _, ctx) => {
      if (ctx.session?.userId !== self.id || !parseAllowlist(env.PRISM_BETA_USER_IDS).includes(self.id)) return null;
      return prismCommands();
    },
  }),

  prismSessions: t.field({
    type: [PrismSession],
    args: { includeArchived: t.arg.boolean({ defaultValue: false }) },
    resolve: async (self, args, ctx) => {
      if (!ctx.session || ctx.session.userId !== self.id) throw new TypieError({ code: 'forbidden', status: 403 });
      return db
        .select()
        .from(PrismSessions)
        .where(
          and(
            eq(PrismSessions.userId, self.id),
            isNull(PrismSessions.deletedAt),
            args.includeArchived ? undefined : isNull(PrismSessions.archivedAt),
          ),
        )
        .orderBy(desc(PrismSessions.updatedAt));
    },
  }),
}));

builder.queryFields((t) => ({
  prismSession: t.withAuth({ session: true }).field({
    type: PrismSession,
    args: { sessionId: t.arg.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: (_, args, ctx) => ownedSession(args.sessionId, ctx.session.userId),
  }),

  prismEntities: t.withAuth({ session: true }).field({
    type: [Entity],
    args: { ids: t.arg.idList() },
    resolve: async (_, args, ctx) => {
      const ids = [...new Set(args.ids)];
      if (ids.length === 0) return [];

      const rows = await db
        .select({ entity: Entities })
        .from(Entities)
        .leftJoin(Documents, eq(Documents.entityId, Entities.id))
        .leftJoin(Folders, eq(Folders.entityId, Entities.id))
        .where(entityRefFilter(ids));
      const entities = [...new Map(rows.map((row) => [row.entity.id, row.entity])).values()];

      const privateSiteIds = [
        ...new Set(entities.filter((entity) => entity.availability === EntityAvailability.PRIVATE).map((entity) => entity.siteId)),
      ];
      await Promise.all(
        privateSiteIds.map((siteId) =>
          assertSitePermission({ userId: ctx.session.userId, siteId }).catch(() => {
            throw new NotFoundError();
          }),
        ),
      );

      return entities;
    },
  }),

  prismNotes: t.withAuth({ session: true }).field({
    type: [Note],
    args: { ids: t.arg.idList() },
    resolve: async (_, args, ctx) => {
      const ids = [...new Set(args.ids)];
      if (ids.length === 0) return [];

      return await db
        .select()
        .from(Notes)
        .where(and(inArray(Notes.id, ids), eq(Notes.userId, ctx.session.userId)));
    },
  }),
}));

const SendPrismMessageResult = builder.simpleObject('SendPrismMessageResult', {
  fields: (t) => ({ session: t.field({ type: PrismSession }), runSeq: t.int() }),
});

builder.mutationFields((t) => ({
  sendPrismMessage: t.withAuth({ session: true }).fieldWithInput({
    type: SendPrismMessageResult,
    input: {
      sessionId: t.input.id({ required: false, validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      siteId: t.input.id({ required: false, validate: validateDbId(TableCode.SITES) }),
      toolPolicy: t.input.field({ type: PrismToolPolicy, required: false }),
      message: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId, credit: { required: 1 } });
      const message = input.message.trim();
      if (message.length === 0) throw new TypieError({ code: 'empty_message', status: 400 });
      const key = nanoid();

      const attachRunSite = async (sessionId: string, runSeq: number) => {
        if (!input.siteId) return;
        await assertSitePermission({ userId: ctx.session.userId, siteId: input.siteId });
        await db
          .insert(PrismRuns)
          .values({ sessionId, runSeq, siteId: input.siteId, startedAt: dayjs() })
          .onConflictDoUpdate({ target: [PrismRuns.sessionId, PrismRuns.runSeq], set: { siteId: input.siteId } });
      };

      if (input.sessionId) {
        const session = await ownedSession(input.sessionId, ctx.session.userId);
        const { runSeq } = await prism.resumeAgent(session.prismAgentId, { message, key }).catch(prismError);
        if (session.openRunSeq !== null && session.openRunSeq !== runSeq) {
          await closeRun(db, session.id, session.openRunSeq);
          pubsub.publish('prism:credit', ctx.session.userId, {});
        }
        const updated = await db
          .update(PrismSessions)
          .set({ updatedAt: dayjs(), openRunSeq: runSeq })
          .where(eq(PrismSessions.id, session.id))
          .returning()
          .then(firstOrThrow);
        await attachRunSite(session.id, runSeq);
        await ensureIngest({ kind: 'agent', sessionId: session.id });
        return { session: updated, runSeq };
      }
      const agentId = newAgentId();
      const { runSeq } = await prism.invokeAgent({ agentId, message, key, metadata: { userId: ctx.session.userId } }).catch(prismError);
      const session = await db
        .insert(PrismSessions)
        .values({
          userId: ctx.session.userId,
          prismAgentId: agentId,
          lane,
          openRunSeq: runSeq,
          ...(input.toolPolicy && { toolPolicy: input.toolPolicy }),
        })
        .returning()
        .then(firstOrThrow);
      await attachRunSite(session.id, runSeq);
      await ensureIngest({ kind: 'agent', sessionId: session.id });
      return { session, runSeq };
    },
  }),

  resolvePrismTool: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: {
      sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      agentId: t.input.string({ required: false }),
      toolCallId: t.input.string(),
      input: t.input.field({ type: 'JSON' }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });
      const session = await ownedSession(input.sessionId, ctx.session.userId);

      let agentId = session.prismAgentId;
      let childWorkflowId: string | null = null;
      if (input.agentId && input.agentId !== session.prismAgentId) {
        const child = await db
          .select({ id: PrismWorkflows.id })
          .from(PrismWorkflows)
          .innerJoin(PrismWorkflowEvents, eq(PrismWorkflowEvents.workflowId, PrismWorkflows.id))
          .where(
            and(
              eq(PrismWorkflows.sessionId, session.id),
              eq(PrismWorkflows.state, 'RUNNING'),
              eq(PrismWorkflowEvents.kind, 'invocation.started'),
              sql`${PrismWorkflowEvents.data} -> 'target' ->> 'kind' = 'agent'`,
              sql`${PrismWorkflowEvents.data} -> 'target' ->> 'id' = ${input.agentId}`,
            ),
          )
          .then(first);
        if (!child) throw new TypieError({ code: 'not_found', status: 404 });
        agentId = input.agentId;
        childWorkflowId = child.id;
      }

      const agent = await prism.getAgent(agentId).catch(prismError);
      if (!agent.pending || agent.pending.toolCallId !== input.toolCallId) {
        throw new TypieError({ code: 'prism_tool_settled', status: 409 });
      }

      const tool = agent.pending.tool;
      const meta = TOOL_META[tool];
      if (meta?.resolver === 'server') throw new TypieError({ code: 'prism_tool_not_resolvable', status: 400 });

      const handler = prismTools[tool];
      let result = input.input;
      if (handler) {
        const runSeq = agentId === session.prismAgentId ? (activeRun(agent.runs)?.runSeq ?? null) : null;
        const siteId = await runSite(session, runSeq);
        if (siteId === null) throw new TypieError({ code: 'site_not_found', status: 404 });
        const context = { userId: ctx.session.userId, session, siteId, toolCallId: input.toolCallId, agent, afterCommit: undefined };

        if (meta?.tier === 'destructive') {
          if (serveVerdict(tool, session.toolPolicy) === 'deny') throw new TypieError({ code: 'prism_tool_policy', status: 403 });
          const decision = ApproveInputSchema.safeParse(input.input);
          if (!decision.success) throw new TypieError({ code: 'invalid_input', status: 400 });
          const call = { toolCallId: input.toolCallId, tool, resolver: PrismToolResolver.USER };
          if (decision.data.approve) {
            const pendingData = agent.pending.data;
            try {
              result = await withToolLedger(session, call, (tx, afterCommit) =>
                handler({ ...context, executor: tx, afterCommit }, pendingData),
              );
            } catch (err) {
              log.warn('prism tool handler failed: {tool} {*}', { tool, error: err });
              result = toolFailure('error', ERROR_MESSAGE);
            }
          } else {
            result = toolFailure('declined', DECLINED_MESSAGE);
            await recordToolResolution(session, call, result);
          }
        } else {
          try {
            result = await handler({ ...context, executor: db }, input.input);
          } catch (err) {
            if (err instanceof TypieError) throw err;
            prismError(err);
          }
        }
      } else if (meta?.tier === 'destructive') {
        throw new TypieError({ code: 'prism_tool_not_resolvable', status: 400 });
      }

      await prism.resolveTool(agentId, input.toolCallId, result).catch(prismError);

      const running = agentId === session.prismAgentId ? activeRun(agent.runs) : null;

      const updated = await db
        .update(PrismSessions)
        .set(running ? { updatedAt: dayjs(), openRunSeq: running.runSeq } : { updatedAt: dayjs() })
        .where(eq(PrismSessions.id, session.id))
        .returning()
        .then(firstOrThrow);

      await ensureIngest({ kind: 'agent', sessionId: session.id });
      if (childWorkflowId !== null) await ensureIngest({ kind: 'workflow', workflowId: childWorkflowId });

      return updated;
    },
  }),

  cancelPrismRun: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });
      const session = await ownedSession(input.sessionId, ctx.session.userId);
      await cancelSessionWorkflows(session.id);
      await cancelActiveRun(session.prismAgentId).catch(prismError);
      await ensureIngest({ kind: 'agent', sessionId: session.id });
      return session;
    },
  }),

  archivePrismSession: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      await ownedSession(input.sessionId, ctx.session.userId);
      return db
        .update(PrismSessions)
        .set({ archivedAt: dayjs() })
        .where(eq(PrismSessions.id, input.sessionId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  unarchivePrismSession: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      await ownedSession(input.sessionId, ctx.session.userId);
      return db.update(PrismSessions).set({ archivedAt: null }).where(eq(PrismSessions.id, input.sessionId)).returning().then(firstOrThrow);
    },
  }),

  renamePrismSession: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: {
      sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      title: t.input.string({ validate: { schema: prismSchema.title } }),
    },
    resolve: async (_, { input }, ctx) => {
      await ownedSession(input.sessionId, ctx.session.userId);
      return db
        .update(PrismSessions)
        .set({ title: input.title.trim() })
        .where(eq(PrismSessions.id, input.sessionId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  updatePrismSessionToolPolicy: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: {
      sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      policy: t.input.field({ type: PrismToolPolicy }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });
      const session = await ownedSession(input.sessionId, ctx.session.userId);

      const updated = await db
        .update(PrismSessions)
        .set({ toolPolicy: input.policy })
        .where(eq(PrismSessions.id, session.id))
        .returning()
        .then(firstOrThrow);

      await ensureIngest({ kind: 'agent', sessionId: session.id }).catch((err: unknown) => {
        log.warn('prism ingest wake failed after policy change: {*}', { error: err });
      });

      return updated;
    },
  }),

  deletePrismSession: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      const session = await ownedSession(input.sessionId, ctx.session.userId);
      await cancelSessionWorkflows(session.id);

      let canceled = true;
      try {
        await cancelActiveRun(session.prismAgentId);
      } catch (err) {
        if (!(err instanceof PrismApiError && err.status === 404)) {
          canceled = false;
          log.warn('prism run cancel on delete failed: {sessionId} {*}', { sessionId: session.id, error: err });
        }
      }

      const updated = await db
        .update(PrismSessions)
        .set({ deletedAt: dayjs() })
        .where(eq(PrismSessions.id, input.sessionId))
        .returning()
        .then(firstOrThrow);

      if (canceled && session.openRunSeq !== null) {
        await closeRun(db, session.id, session.openRunSeq).catch((err) =>
          log.warn('prism close-run on delete failed: {sessionId} {*}', { sessionId: session.id, error: err }),
        );
        pubsub.publish('prism:credit', ctx.session.userId, {});
      }

      return updated;
    },
  }),

  markPrismSessionSeen: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      const session = await ownedSession(input.sessionId, ctx.session.userId);

      const updated = await db
        .update(PrismSessions)
        .set({ seenAt: dayjs() })
        .where(eq(PrismSessions.id, session.id))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('prism:badge', ctx.session.userId, { sessionId: updated.id });

      return updated;
    },
  }),

  reactPrismRun: t.withAuth({ session: true }).fieldWithInput({
    type: PrismRun,
    input: {
      runId: t.input.id({ validate: validateDbId(TableCode.PRISM_RUNS) }),
      reaction: t.input.field({ type: PrismReaction, required: false }),
      note: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const run = await db.select().from(PrismRuns).where(eq(PrismRuns.id, input.runId)).then(first);
      if (!run) throw new TypieError({ code: 'not_found', status: 404 });
      const session = await ownedSession(run.sessionId, ctx.session.userId);

      const updated = await db
        .update(PrismRuns)
        .set({ reaction: input.reaction ?? null, reactionNote: input.reaction ? input.note?.trim() || null : null })
        .where(eq(PrismRuns.id, run.id))
        .returning()
        .then(firstOrThrow);

      const transcript = await loadTranscript(session);
      const shaped = transcript.runs.find((r) => r.row.id === updated.id);
      return shaped ?? { runSeq: updated.runSeq, items: [], row: updated };
    },
  }),
}));

builder.subscriptionFields((t) => ({
  prismBadgeStream: t.withAuth({ session: true }).field({
    type: PrismSession,
    subscribe: async (_, __, ctx) => pubsub.subscribe('prism:badge', ctx.session.userId),
    resolve: (payload, _, ctx) => {
      // 구독 ctx는 소켓 수명이다 — 로더를 비우지 않으면 두 번째 이벤트부터 첫 값이 굳는다
      clearLoaders(ctx);
      return payload.sessionId;
    },
  }),

  prismCreditStream: t.withAuth({ session: true }).field({
    type: User,
    subscribe: async (_, __, ctx) => pubsub.subscribe('prism:credit', ctx.session.userId),
    resolve: (_, __, ctx) => {
      clearLoaders(ctx);
      return ctx.session.userId;
    },
  }),

  prismSessionEvents: t.withAuth({ session: true }).field({
    type: 'JSON',
    args: {
      sessionId: t.arg.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      cursor: t.arg.int({ required: false }),
      workflows: t.arg({ type: [PrismWorkflowCursorInput], required: false }),
    },
    subscribe: async (_, args, ctx) => {
      const session = await ownedSession(args.sessionId, ctx.session.userId);
      await ensureIngest({ kind: 'agent', sessionId: session.id });

      const cursor = args.cursor ?? 0;
      const workflowCursors = new Map((args.workflows ?? []).map((workflow) => [workflow.workflowId, workflow.cursor]));

      return new Repeater<ProjectedStreamFrame>(async (push, stop) => {
        let stopped = false;
        void stop.then(() => {
          stopped = true;
        });

        const live = pubsub.subscribe('prism:session', session.id);
        void stop.then(() => live.return());

        const gate = createFrameGate(cursor, workflowCursors);
        const watermarks = new Map<string, { context: TurnContext; length: number }>();
        const buffered: ProjectedStreamFrame[] = [];
        let replaying = true;

        const fieldKeyOf = (delta: ProjectedDeltaFrame) =>
          liveFieldKey(
            delta.workflowId === undefined ? { source: 'SESSION' } : { source: 'WORKFLOW', workflowId: delta.workflowId },
            delta.context.agent.id,
          );

        const turnOrder = (a: TurnContext, b: TurnContext): number =>
          a.run === b.run ? (a.turn === b.turn ? a.attempt - b.attempt : a.turn - b.turn) : a.run - b.run;

        const rewound = (delta: ProjectedDeltaFrame): boolean => {
          if (delta.channel !== 'text') return false;
          const key = fieldKeyOf(delta);
          const mark = watermarks.get(key);
          if (mark === undefined) return false;

          const order = turnOrder(delta.context, mark.context);
          if (order > 0) return false;
          if (order < 0) return true;
          if (delta.offset === 0) {
            watermarks.set(key, { context: delta.context, length: delta.data.length });
            return false;
          }

          return delta.offset < mark.length;
        };

        const accept = (frame: ProjectedStreamFrame): boolean =>
          frame.type === 'delta' && rewound(frame.delta) ? false : gate.accept(frame);

        const drain = (async () => {
          for await (const frame of live) {
            if (replaying) buffered.push(frame);
            else if (accept(frame)) await push(frame);
          }
        })().catch((err: unknown) => stop(toTypieError(err)));

        try {
          const events = await db
            .select()
            .from(PrismSessionEvents)
            .where(and(eq(PrismSessionEvents.sessionId, session.id), gt(PrismSessionEvents.seq, cursor)))
            .orderBy(asc(PrismSessionEvents.seq));
          const workflows = await db.select().from(PrismWorkflows).where(eq(PrismWorkflows.sessionId, session.id));
          const workflowRows =
            workflows.length === 0
              ? []
              : await db
                  .select()
                  .from(PrismWorkflowEvents)
                  .where(
                    inArray(
                      PrismWorkflowEvents.workflowId,
                      workflows.map((workflow) => workflow.id),
                    ),
                  )
                  .orderBy(asc(PrismWorkflowEvents.seq));
          const prismIdOf = new Map(workflows.map((workflow) => [workflow.id, workflow.prismWorkflowId]));

          for (const event of storedEvents(events)) {
            if (event.context === null) continue;
            const frame = projectFrame({ type: 'event', event }, { source: 'SESSION' });
            if (frame !== null && accept(frame)) await push(frame);
          }

          for (const row of workflowRows) {
            const workflowId = prismIdOf.get(row.workflowId);
            if (workflowId === undefined || row.seq <= (workflowCursors.get(workflowId) ?? 0)) continue;
            const [event] = storedEvents([row]);
            if (event.context === null) continue;
            const frame = projectFrame({ type: 'event', event }, { source: 'WORKFLOW', workflowId });
            if (frame !== null && accept(frame)) await push(frame);
          }

          await push({ type: 'sync', seq: Math.max(cursor, session.cursor, events.at(-1)?.seq ?? 0) });

          const snapshot = await redis.hgetall(`prism:live:${session.id}`);
          const fields: Record<string, TurnLive> = {};
          for (const [field, json] of Object.entries(snapshot)) {
            try {
              fields[field] = JSON.parse(json) as TurnLive;
            } catch (err) {
              log.warn('prism live snapshot field unparsable: {sessionId} {field} {*}', { sessionId: session.id, field, error: err });
            }
          }

          for (const delta of liveSnapshotFrames(fields)) {
            // 재생에서 이미 봉인된 턴의 스냅샷(이전 펌프 인스턴스가 남긴 stale 필드)은 게이트가 거른다
            if (!gate.accept({ type: 'delta', delta })) continue;
            if (delta.channel === 'text') {
              watermarks.set(fieldKeyOf(delta), { context: delta.context, length: delta.offset + delta.data.length });
            }
            await push({ type: 'delta', delta });
          }

          for (;;) {
            const pending = [...buffered];
            buffered.length = 0;
            if (pending.length === 0) {
              replaying = false;
              break;
            }
            for (const frame of pending) if (accept(frame)) await push(frame);
          }

          await drain;
          stop();
        } catch (err) {
          if (stopped) stop();
          else stop(toTypieError(err));
        }
      });
    },
    resolve: (payload) => payload,
  }),
}));

import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { PrismWorkflowState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { prismSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { and, desc, eq, isNull } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import { nanoid } from 'nanoid';
import { db, first, firstOrThrow, PrismSessions, PrismWorkflows, TableCode, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { activeRun, newAgentId, prism, PrismApiError } from '#/external/prism.ts';
import { pumpSse } from '#/external/prism-stream.ts';
import { assertPrismAccess } from '#/utils/prism-access.ts';
import { parseAllowlist } from '#/utils/prism-access-core.ts';
import { prismApps } from '#/utils/prism-apps.ts';
import { prismCommands } from '#/utils/prism-catalog.ts';
import { projectFrame } from '#/utils/prism-events.ts';
import { routeSessionFrame } from '#/utils/prism-session-stream.ts';
import { prismTools } from '#/utils/prism-tools.ts';
import { cancelActiveRun, cancelSessionWorkflows, closeRun, linkWorkflow, settleWorkflow, titleSession } from '#/utils/prism-workflows.ts';
import { isRunningChildAgent } from '#/utils/prism-workflows-core.ts';
import { builder } from '../builder.ts';
import { PrismSession, PrismWorkflow, User } from '../objects.ts';
import type { ProjectedStreamFrame, StreamFrame } from '@typie/prism';
import type { PrismWorkflowRow } from '#/utils/prism-apps.ts';
import type { ProjectedScope } from '#/utils/prism-events.ts';

const log = logger.getChild('prism');

const IDLE_MS = 45_000;
const RECONNECT_DELAY_MS = 1000;

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

PrismSession.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.exposeString('title', { nullable: true }),
    archivedAt: t.expose('archivedAt', { type: 'DateTime', nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
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

  prismSessionLog: t.withAuth({ session: true }).field({
    type: ['JSON'],
    args: {
      sessionId: t.arg.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      cursor: t.arg.int({ defaultValue: 0 }),
    },
    resolve: async (_, args, ctx) => {
      const session = await ownedSession(args.sessionId, ctx.session.userId);
      const { events, sync } = await prism
        .readAgentEventsUntilSync(session.prismAgentId, args.cursor, new AbortController().signal)
        .catch(prismError);
      const titled = events
        .map((event) => routeSessionFrame({ type: 'event', event }))
        .findLast((route): route is { kind: 'titled'; title: string } => route?.kind === 'titled');
      if (titled && titled.title !== session.title) await titleSession(session.id, titled.title);
      const projected = events
        .map((event) => projectFrame({ type: 'event', event }, { source: 'SESSION' }))
        .filter((frame) => frame !== null);
      return [...projected, { type: 'sync', seq: sync } satisfies ProjectedStreamFrame];
    },
  }),

  prismWorkflowLog: t.withAuth({ session: true }).field({
    type: ['JSON'],
    args: {
      workflowId: t.arg.string(),
    },
    resolve: async (_, args, ctx) => {
      let workflow = await db.select().from(PrismWorkflows).where(eq(PrismWorkflows.prismWorkflowId, args.workflowId)).then(first);

      if (workflow) {
        await ownedSession(workflow.sessionId, ctx.session.userId);
      } else {
        const state = await prism.getWorkflow(args.workflowId).catch(() => null);
        const app = state?.workflow.app ?? null;
        const resolve = app === null ? undefined : prismApps[app]?.resolveSession;
        const sessionId = resolve === undefined || state === null ? null : await resolve(state.workflow.ref);
        if (sessionId === null) throw new TypieError({ code: 'not_found', status: 404 });
        await ownedSession(sessionId, ctx.session.userId);
        workflow = await linkWorkflow(sessionId, args.workflowId);
      }

      const { events } = await prism
        .readWorkflowEventsUntilSync(workflow.prismWorkflowId, 0, new AbortController().signal)
        .catch(prismError);
      return events
        .map((event) => projectFrame({ type: 'event', event }, { source: 'WORKFLOW', workflowId: workflow.prismWorkflowId }))
        .filter((frame): frame is ProjectedStreamFrame => frame !== null);
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
      message: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });
      const message = input.message.trim();
      if (message.length === 0) throw new TypieError({ code: 'empty_message', status: 400 });
      const key = nanoid();
      if (input.sessionId) {
        const session = await ownedSession(input.sessionId, ctx.session.userId);
        const { runSeq } = await prism.resumeAgent(session.prismAgentId, { message, key }).catch(prismError);
        if (session.openRunSeq !== null && session.openRunSeq !== runSeq) await closeRun(session.id, session.openRunSeq);
        const updated = await db
          .update(PrismSessions)
          .set({ updatedAt: dayjs(), openRunSeq: runSeq })
          .where(eq(PrismSessions.id, session.id))
          .returning()
          .then(firstOrThrow);
        return { session: updated, runSeq };
      }
      const agentId = newAgentId();
      const { runSeq } = await prism.invokeAgent({ agentId, message, key, metadata: { userId: ctx.session.userId } }).catch(prismError);
      const session = await db
        .insert(PrismSessions)
        .values({ userId: ctx.session.userId, prismAgentId: agentId, openRunSeq: runSeq })
        .returning()
        .then(firstOrThrow);
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
      if (input.agentId && input.agentId !== session.prismAgentId) {
        const rows = await db
          .select({ prismWorkflowId: PrismWorkflows.prismWorkflowId })
          .from(PrismWorkflows)
          .where(and(eq(PrismWorkflows.sessionId, session.id), eq(PrismWorkflows.state, 'RUNNING')));

        let allowed = false;
        let looked = false;
        let lookupError: unknown = null;
        for (const row of rows) {
          try {
            const { invocations } = await prism.getWorkflow(row.prismWorkflowId);
            looked = true;
            if (isRunningChildAgent(invocations, input.agentId)) {
              allowed = true;
              break;
            }
          } catch (err) {
            lookupError = err;
            log.warn('workflow lookup failed for {id}: {*}', { id: row.prismWorkflowId, error: err });
          }
        }
        if (!allowed && !looked && lookupError !== null) prismError(lookupError);
        if (!allowed) throw new TypieError({ code: 'not_found', status: 404 });
        agentId = input.agentId;
      }

      const agent = await prism.getAgent(agentId).catch(prismError);
      if (!agent.pending || agent.pending.toolCallId !== input.toolCallId) {
        throw new TypieError({ code: 'prism_tool_settled', status: 409 });
      }

      const handler = prismTools[agent.pending.tool];
      let result = input.input;
      if (handler) {
        try {
          result = await handler({ userId: ctx.session.userId, session, toolCallId: input.toolCallId, agent }, input.input);
        } catch (err) {
          if (err instanceof TypieError) throw err;
          prismError(err);
        }
      }

      await prism.resolveTool(agentId, input.toolCallId, result).catch(prismError);

      const running = agentId === session.prismAgentId ? activeRun(agent.runs) : null;

      return db
        .update(PrismSessions)
        .set(running ? { updatedAt: dayjs(), openRunSeq: running.runSeq } : { updatedAt: dayjs() })
        .where(eq(PrismSessions.id, session.id))
        .returning()
        .then(firstOrThrow);
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
        await closeRun(session.id, session.openRunSeq).catch((err) =>
          log.warn('prism close-run on delete failed: {sessionId} {*}', { sessionId: session.id, error: err }),
        );
      }

      return updated;
    },
  }),
}));

builder.subscriptionFields((t) => ({
  prismSessionEvents: t.withAuth({ session: true }).field({
    type: 'JSON',
    args: {
      sessionId: t.arg.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      cursor: t.arg.int({ required: false }),
      workflows: t.arg({ type: [PrismWorkflowCursorInput], required: false }),
    },
    subscribe: async (_, args, ctx) => {
      const session = await ownedSession(args.sessionId, ctx.session.userId);
      const workflowCursors = new Map((args.workflows ?? []).map((workflow) => [workflow.workflowId, workflow.cursor]));
      return new Repeater<ProjectedStreamFrame>(async (push, stop) => {
        const controller = new AbortController();
        void stop.then(() => controller.abort());
        const attached = new Set<string>();
        const pumps: Promise<void>[] = [];

        const pumpUpstream = async (opts: {
          open: (cursor: number) => Promise<ReadableStream<Uint8Array>>;
          scope: ProjectedScope;
          cursor: number;
          onFrame: (frame: StreamFrame) => Promise<void>;
          isTerminal: () => boolean;
        }) => {
          let cursor = opts.cursor;
          while (!controller.signal.aborted) {
            try {
              const stream = await opts.open(cursor);
              let seeding = false;
              const seeded = new Set<string>();
              const outcome = await pumpSse({
                stream,
                idleMs: IDLE_MS,
                signal: controller.signal,
                onFrame: async (frame) => {
                  if (frame.type === 'event') {
                    cursor = frame.event.seq;
                    seeding = false;
                  } else if (frame.type === 'sync') {
                    seeding = true;
                    seeded.clear();
                  }
                  await opts.onFrame(frame);
                  let projected = projectFrame(frame, opts.scope);
                  if (seeding && frame.type === 'delta' && projected?.type === 'delta') {
                    const wire = frame.delta;
                    const key = `${wire.context.agent.id}|${wire.channel}|${wire.channel === 'tool.input' ? (wire.tool.id ?? wire.tool.name) : ''}`;
                    if (seeded.has(key)) {
                      seeding = false;
                    } else {
                      seeded.add(key);
                      projected = { ...projected, delta: { ...projected.delta, seed: true } };
                    }
                  }
                  if (projected !== null) await push(projected);
                },
              });
              if (outcome === 'aborted') return;
              if (outcome === 'closed') {
                if (opts.isTerminal()) return;
                log.warn('prism stream closed before terminal, reconnecting: {source} {workflowId} (seq {cursor})', {
                  source: opts.scope.source,
                  workflowId: opts.scope.source === 'WORKFLOW' ? opts.scope.workflowId : null,
                  cursor,
                });
              }
            } catch (err) {
              if (controller.signal.aborted) return;
              if (!(err instanceof PrismApiError) || err.status < 500) throw err;
              log.warn('prism stream reconnect after upstream failure: {code} ({status})', { code: err.code, status: err.status });
            }
            await new Promise((resolve) => setTimeout(resolve, RECONNECT_DELAY_MS));
          }
        };

        const attachWorkflow = (row: PrismWorkflowRow) => {
          const workflowId = row.prismWorkflowId;
          if (attached.has(workflowId)) return;
          attached.add(workflowId);
          let terminalSeen = false;
          pumps.push(
            pumpUpstream({
              open: (cursor) => prism.openWorkflowEvents(workflowId, cursor, controller.signal),
              scope: { source: 'WORKFLOW', workflowId },
              cursor: workflowCursors.get(workflowId) ?? 0,
              isTerminal: () => terminalSeen,
              onFrame: async (frame) => {
                if (routeSessionFrame(frame)?.kind === 'workflow-terminal') {
                  terminalSeen = true;
                  await prism
                    .getWorkflow(workflowId)
                    .then((state) => settleWorkflow(row, state))
                    .catch((err) => log.error('prism workflow settle failed {workflowId} {*}', { workflowId, error: err }));
                }
              },
            }).catch((err) => {
              if (controller.signal.aborted) return;
              log.error('prism workflow stream stopped: {workflowId} {*}', { workflowId, error: err });
            }),
          );
        };

        try {
          const running = await db
            .select()
            .from(PrismWorkflows)
            .where(and(eq(PrismWorkflows.sessionId, session.id), eq(PrismWorkflows.state, 'RUNNING')));
          for (const row of running) {
            attachWorkflow(row);
          }

          await pumpUpstream({
            open: (cursor) => prism.openAgentEvents(session.prismAgentId, cursor, controller.signal),
            scope: { source: 'SESSION' },
            cursor: args.cursor ?? 0,
            isTerminal: () => false,
            onFrame: async (frame) => {
              const route = routeSessionFrame(frame);
              if (route?.kind === 'workflow-started') {
                try {
                  const row = await linkWorkflow(session.id, route.workflowId);
                  if (row.sessionId !== session.id) {
                    log.warn('prism workflow belongs to another session: {workflowId}', { workflowId: route.workflowId });
                    return;
                  }
                  attachWorkflow(row);
                } catch (err) {
                  log.warn('prism workflow attach failed: {workflowId} {*}', { workflowId: route.workflowId, error: err });
                }
              } else if (route?.kind === 'invocation-retried' && frame.type === 'event') {
                log.warn('prism invocation retried, not reattaching (seq {seq})', { seq: frame.event.seq });
              } else if (route?.kind === 'run-terminal') {
                await closeRun(session.id, route.runSeq).catch((err) =>
                  log.warn('prism close-run failed: {runSeq} {*}', { runSeq: route.runSeq, error: err }),
                );
              } else if (route?.kind === 'titled') {
                await titleSession(session.id, route.title).catch((err) => log.warn('prism title failed: {*}', { error: err }));
              }
            },
          });
          await Promise.all(pumps);
          stop();
        } catch (err) {
          if (controller.signal.aborted) stop();
          else stop(toTypieError(err));
        }
      });
    },
    resolve: (payload) => payload,
  }),
}));

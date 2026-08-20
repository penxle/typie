import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { TypieError } from '@typie/lib/errors';
import { prismSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { and, desc, eq, isNull } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import { nanoid } from 'nanoid';
import { db, first, firstOrThrow, PrismSessions, TableCode, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { activeRun, newAgentId, prism, PrismApiError, sessionTitleFrom } from '#/external/prism.ts';
import { pumpSse } from '#/external/prism-stream.ts';
import { assertPrismAccess } from '#/utils/prism-access.ts';
import { parseAllowlist } from '#/utils/prism-access-core.ts';
import { projectFrame } from '#/utils/prism-events.ts';
import { builder } from '../builder.ts';
import { PrismSession, User } from '../objects.ts';
import type { ProjectedStreamFrame } from '@typie/prism';

const log = logger.getChild('prism');

const IDLE_MS = 45_000;
const RECONNECT_DELAY_MS = 1000;

const toTypieError = (err: unknown): TypieError => {
  if (err instanceof PrismApiError) {
    log.warn('prism-api rejected: {code} ({status})', { code: err.code, status: err.status });
    if (err.code === 'run-active') return new TypieError({ code: 'prism_run_active', status: 409 });
    if (err.status >= 500 || err.code === 'internal' || err.code === 'malformed-response')
      return new TypieError({ code: 'prism_unavailable', status: 502 });
    return new TypieError({ code: `prism_rejected:${err.code}`, status: err.status });
  }
  log.error('prism call failed {*}', { error: err });
  Sentry.captureException(err);
  return new TypieError({ code: 'prism_unavailable', status: 502 });
};

const prismError = (err: unknown): never => {
  throw toTypieError(err);
};

const ownedSession = async (sessionId: string, userId: string) => {
  const session = await db
    .select()
    .from(PrismSessions)
    .where(and(eq(PrismSessions.id, sessionId), eq(PrismSessions.userId, userId), isNull(PrismSessions.deletedAt)))
    .then(first);
  if (!session) throw new TypieError({ code: 'not_found', status: 404 });
  return session;
};

PrismSession.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.exposeString('title', { nullable: true }),
    archivedAt: t.expose('archivedAt', { type: 'DateTime', nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

builder.objectFields(User, (t) => ({
  prismAccess: t.boolean({
    resolve: (self, _, ctx) => ctx.session?.userId === self.id && parseAllowlist(env.PRISM_BETA_USER_IDS).includes(self.id),
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
      const projected = events.map((event) => projectFrame({ type: 'event', event })).filter((frame) => frame !== null);
      return [...projected, { type: 'sync', seq: sync } satisfies ProjectedStreamFrame];
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
        const updated = await db
          .update(PrismSessions)
          .set({ updatedAt: dayjs() })
          .where(eq(PrismSessions.id, session.id))
          .returning()
          .then(firstOrThrow);
        return { session: updated, runSeq };
      }
      const agentId = newAgentId();
      const { runSeq } = await prism.invokeAgent({ agentId, message, key, metadata: { userId: ctx.session.userId } }).catch(prismError);
      const session = await db
        .insert(PrismSessions)
        .values({ userId: ctx.session.userId, prismAgentId: agentId, title: sessionTitleFrom(message) })
        .returning()
        .then(firstOrThrow);
      return { session, runSeq };
    },
  }),

  cancelPrismRun: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });
      const session = await ownedSession(input.sessionId, ctx.session.userId);
      const agent = await prism.getAgent(session.prismAgentId).catch(prismError);
      const running = activeRun(agent.runs);
      if (running) await prism.cancelAgentRun(session.prismAgentId, running.runSeq).catch(prismError);
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

  // 개명은 updatedAt을 건드리지 않는다 — 목록 정렬·그룹은 "마지막 대화" 기준이고 제목 정리는 대화가 아니다.
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

  // soft delete(오너 결정) — 행은 남고 목록·소유 검사에서만 사라진다. prism 측 에이전트 DO는 건드리지 않는다.
  deletePrismSession: t.withAuth({ session: true }).fieldWithInput({
    type: PrismSession,
    input: { sessionId: t.input.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }) },
    resolve: async (_, { input }, ctx) => {
      await ownedSession(input.sessionId, ctx.session.userId);
      return db
        .update(PrismSessions)
        .set({ deletedAt: dayjs() })
        .where(eq(PrismSessions.id, input.sessionId))
        .returning()
        .then(firstOrThrow);
    },
  }),
}));

builder.subscriptionFields((t) => ({
  prismSessionEvents: t.withAuth({ session: true }).field({
    type: 'JSON',
    args: {
      sessionId: t.arg.id({ validate: validateDbId(TableCode.PRISM_SESSIONS) }),
      cursor: t.arg.int({ required: false }),
    },
    subscribe: async (_, args, ctx) => {
      const session = await ownedSession(args.sessionId, ctx.session.userId);
      return new Repeater<ProjectedStreamFrame>(async (push, stop) => {
        const controller = new AbortController();
        void stop.then(() => controller.abort());
        let cursor = args.cursor ?? 0;
        try {
          while (!controller.signal.aborted) {
            try {
              const stream = await prism.openAgentEvents(session.prismAgentId, cursor, controller.signal);
              const outcome = await pumpSse({
                stream,
                idleMs: IDLE_MS,
                signal: controller.signal,
                onFrame: async (frame) => {
                  if (frame.type === 'event') cursor = frame.event.seq;
                  const projected = projectFrame(frame);
                  if (projected !== null) await push(projected);
                },
              });
              if (outcome === 'aborted') break;
            } catch (err) {
              if (controller.signal.aborted) break;
              if (!(err instanceof PrismApiError) || err.status < 500) throw err;
              log.warn('prism stream reconnect after upstream failure: {code} ({status})', { code: err.code, status: err.status });
            }
            await new Promise((resolve) => setTimeout(resolve, RECONNECT_DELAY_MS));
          }
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

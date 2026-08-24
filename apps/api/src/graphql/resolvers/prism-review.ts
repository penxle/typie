import {
  PrismReaction,
  PrismReviewCommentAuthor,
  PrismReviewPass,
  PrismReviewRoundState,
  PrismReviewThreadState,
  PrismReviewTier,
} from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, count, desc, eq, isNotNull, lt, ne, sql } from 'drizzle-orm';
import { clearLoaders } from '#/context.ts';
import {
  db,
  first,
  firstOrThrow,
  PrismReviewRounds,
  PrismReviewThreadComments,
  PrismReviewThreads,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { assertDocumentPermission } from '#/utils/permission.ts';
import { detailOutcome, hasDetail, roundState, summarizeOutcome, threadQuote } from '#/utils/prism-review-core.ts';
import { clearRoundMemos, ensureRoundThreads, roundThreadComments, roundVersionContent } from '#/utils/prism-review-threads.ts';
import { builder } from '../builder.ts';
import {
  Document,
  PrismReviewRound,
  PrismReviewThread,
  PrismReviewThreadComment,
  PrismSession,
  PrismWorkflow,
  UserView,
} from '../objects.ts';

const PrismReviewRejection = builder.simpleObject('PrismReviewRejection', { fields: (t) => ({ message: t.string() }) });

const PrismReviewConclusionSummary = builder.simpleObject('PrismReviewConclusionSummary', {
  fields: (t) => ({
    understanding: t.string({ nullable: true }),
    progress: t.string({ nullable: true }),
    strengthsCount: t.int(),
    verdictsCount: t.int(),
    elevationsCount: t.int(),
    patternsCount: t.int(),
    prioritiesCount: t.int(),
  }),
});

const PrismReviewIssueBrief = builder.simpleObject('PrismReviewIssueBrief', {
  fields: (t) => ({ index: t.int(), trait: t.string() }),
});

const PrismReviewAnchor = builder.simpleObject('PrismReviewAnchor', {
  fields: (t) => ({ start: t.int(), end: t.int(), head: t.string(), tail: t.string() }),
});

const PrismReviewStrength = builder.simpleObject('PrismReviewStrength', {
  fields: (t) => ({
    quote: t.string(),
    body: t.string({ nullable: true }),
    anchor: t.field({ type: PrismReviewAnchor }),
  }),
});

const PrismReviewVerdict = builder.simpleObject('PrismReviewVerdict', {
  fields: (t) => ({ trait: t.string(), note: t.string({ nullable: true }) }),
});

const PrismReviewElevation = builder.simpleObject('PrismReviewElevation', {
  fields: (t) => ({ trait: t.string(), quote: t.string({ nullable: true }), body: t.string() }),
});

const PrismReviewPattern = builder.simpleObject('PrismReviewPattern', {
  fields: (t) => ({ theme: t.string({ nullable: true }), body: t.string(), issues: t.field({ type: [PrismReviewIssueBrief] }) }),
});

const PrismReviewPriority = builder.simpleObject('PrismReviewPriority', {
  fields: (t) => ({ body: t.string(), issues: t.field({ type: [PrismReviewIssueBrief] }) }),
});

const PrismReviewDetail = builder.simpleObject('PrismReviewDetail', {
  fields: (t) => ({
    understanding: t.string({ nullable: true }),
    progress: t.string({ nullable: true }),
    strengths: t.field({ type: [PrismReviewStrength] }),
    verdicts: t.field({ type: [PrismReviewVerdict] }),
    elevations: t.field({ type: [PrismReviewElevation] }),
    patterns: t.field({ type: [PrismReviewPattern] }),
    priorities: t.field({ type: [PrismReviewPriority] }),
  }),
});

PrismReviewThreadComment.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    author: t.expose('author', { type: PrismReviewCommentAuthor }),
    user: t.expose('userId', { type: UserView }),
    body: t.exposeString('body'),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

PrismReviewThread.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    issueIndex: t.exposeInt('issueIndex'),
    issueId: t.exposeString('issueId', { nullable: true }),
    trait: t.exposeString('trait'),
    pass: t.expose('pass', { type: PrismReviewPass }),
    body: t.exposeString('body', { nullable: true }),
    anchors: t.field({ type: [PrismReviewAnchor], resolve: (self) => self.anchors }),
    quote: t.string({
      // 리뷰 시점 판본에서 자른다 — 카드는 앵커가 살아 있든 자리를 잃었든 이 인용을 보여 준다
      resolve: async (self, _, ctx) => threadQuote(await roundVersionContent(ctx, self.roundId), self.anchors),
    }),
    state: t.expose('state', { type: PrismReviewThreadState }),
    stateChangedAt: t.expose('stateChangedAt', { type: 'DateTime', nullable: true }),
    reaction: t.expose('reaction', { type: PrismReaction, nullable: true }),
    comments: t.field({
      type: [PrismReviewThreadComment],
      resolve: async (self, _, ctx) => {
        const byThread = await roundThreadComments(ctx, self.roundId);
        return byThread.get(self.id) ?? [];
      },
    }),
  }),
});

const ownedRound = async (roundId: string, userId: string) => {
  const round = await db.select().from(PrismReviewRounds).where(eq(PrismReviewRounds.id, roundId)).then(first);
  if (!round) throw new TypieError({ code: 'not_found', status: 404 });
  await assertDocumentPermission({ userId, documentId: round.documentId });
  return round;
};

PrismReviewRound.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    documentId: t.exposeID('documentId'),
    document: t.expose('documentId', { type: Document }),
    round: t.exposeInt('round'),
    ordinal: t.int({
      resolve: async (self) => {
        const prior = await db
          .select({ count: count() })
          .from(PrismReviewRounds)
          .where(
            and(
              eq(PrismReviewRounds.documentId, self.documentId),
              lt(PrismReviewRounds.round, self.round),
              isNotNull(PrismReviewRounds.result),
              ne(sql`${PrismReviewRounds.result} ->> 'kind'`, 'rejected'),
            ),
          )
          .then(firstOrThrow);
        return prior.count + 1;
      },
    }),
    tier: t.expose('tier', { type: PrismReviewTier }),
    state: t.field({
      type: PrismReviewRoundState,
      resolve: async (self, _, ctx) => {
        const workflow = self.workflowId === null ? null : await PrismWorkflow.getDataloader(ctx).load(self.workflowId);
        return roundState(self, workflow);
      },
    }),
    workflow: t.field({ type: PrismWorkflow, nullable: true, resolve: (self) => self.workflowId }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    rejection: t.field({ type: PrismReviewRejection, nullable: true, resolve: (self) => summarizeOutcome(self.result).rejection }),
    conclusion: t.field({
      type: PrismReviewConclusionSummary,
      nullable: true,
      resolve: (self) => summarizeOutcome(self.result).conclusion,
    }),
    issueCount: t.int({ resolve: (self) => summarizeOutcome(self.result).issueCount }),
    hasDetail: t.boolean({ resolve: (self) => hasDetail(self.result) }),
    detail: t.field({
      type: PrismReviewDetail,
      nullable: true,
      // 인용을 뽑으려면 리뷰 시점 원고 판본이 필요하다 — 전문이 없으면 그 조회도 하지 않는다.
      // 여백 쿼리는 threads.quote와 detail을 함께 고르므로 같은 판본을 요청 안에서 한 번만 읽는다.
      resolve: async (self, _, ctx) => {
        if (!hasDetail(self.result)) return null;
        return detailOutcome(self.result, await roundVersionContent(ctx, self.id));
      },
    }),
    reaction: t.expose('reaction', { type: PrismReaction, nullable: true }),
    reactionNote: t.exposeString('reactionNote', { nullable: true }),
    threads: t.field({
      type: [PrismReviewThread],
      resolve: async (self) => {
        // 배포 전에 끝난 회차는 결과만 있고 행이 없다 — 첫 조회가 메운다
        await ensureRoundThreads(self.id);
        const threads = await db
          .select({ id: PrismReviewThreads.id })
          .from(PrismReviewThreads)
          .where(eq(PrismReviewThreads.roundId, self.id))
          .orderBy(asc(PrismReviewThreads.issueIndex));
        return threads.map((thread) => thread.id);
      },
    }),
  }),
});

builder.objectFields(PrismSession, (t) => ({
  reviewRounds: t.field({
    type: [PrismReviewRound],
    resolve: async (self) => {
      const rounds = await db
        .select({ id: PrismReviewRounds.id })
        .from(PrismReviewRounds)
        .where(eq(PrismReviewRounds.sessionId, self.id))
        .orderBy(asc(PrismReviewRounds.createdAt));
      return rounds.map((round) => round.id);
    },
  }),
}));

builder.objectFields(Document, (t) => ({
  prismReviewRounds: t.field({
    type: [PrismReviewRound],
    // 리뷰는 문서를 읽을 수 있는 사람이 아니라 문서를 가진 사람의 것이다 — UNLISTED 슬러그로 들어온 손님에게는 빈 목록.
    resolve: async (self, _, ctx) => {
      if (!ctx.session) {
        return [];
      }

      try {
        await assertDocumentPermission({ userId: ctx.session.userId, documentId: self.id });
      } catch (err) {
        if (err instanceof TypieError) {
          return [];
        }

        throw err;
      }

      const rounds = await db
        .select({ id: PrismReviewRounds.id, result: PrismReviewRounds.result })
        .from(PrismReviewRounds)
        .where(eq(PrismReviewRounds.documentId, self.id))
        .orderBy(desc(PrismReviewRounds.round));

      // 지적이 있는 완료·비거부 회차만 — 나머지는 여백에 세울 것이 없다
      return rounds.filter((round) => summarizeOutcome(round.result).issueCount > 0).map((round) => round.id);
    },
  }),
}));

builder.queryFields((t) => ({
  prismReviewRound: t.withAuth({ session: true }).field({
    type: PrismReviewRound,
    args: { roundId: t.arg.id({ validate: validateDbId(TableCode.PRISM_REVIEW_ROUNDS) }) },
    resolve: async (_, args, ctx) => {
      const round = await ownedRound(args.roundId, ctx.session.userId);
      return round.id;
    },
  }),
}));

builder.mutationFields((t) => ({
  reactPrismReviewRound: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewRound,
    input: {
      roundId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_ROUNDS) }),
      value: t.input.field({ type: PrismReaction, required: false }),
      note: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const round = await ownedRound(input.roundId, ctx.session.userId);
      const updated = await db
        .update(PrismReviewRounds)
        .set(input.value ? { reaction: input.value, reactionNote: input.note?.trim() || null } : { reaction: null, reactionNote: null })
        .where(eq(PrismReviewRounds.id, round.id))
        .returning()
        .then(firstOrThrow);
      return updated.id;
    },
  }),

  replyPrismReviewThread: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: {
      threadId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREADS) }),
      body: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const thread = await ownedThread(input.threadId, ctx.session.userId);
      const body = input.body.trim();
      if (body.length === 0) {
        throw new TypieError({ code: 'invalid_input', status: 400 });
      }

      await db.insert(PrismReviewThreadComments).values({ threadId: thread.id, author: 'USER', userId: ctx.session.userId, body });
      pubsub.publish('prism:review', thread.documentId, { roundId: thread.roundId });
      return thread.id;
    },
  }),

  updatePrismReviewThreadComment: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: {
      commentId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREAD_COMMENTS) }),
      body: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const { comment, thread } = await ownedComment(input.commentId, ctx.session.userId);
      if (comment.author !== 'USER' || comment.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'permission_denied', status: 403 });
      }

      const body = input.body.trim();
      if (body.length === 0) {
        throw new TypieError({ code: 'invalid_input', status: 400 });
      }

      await db.update(PrismReviewThreadComments).set({ body }).where(eq(PrismReviewThreadComments.id, comment.id));
      pubsub.publish('prism:review', thread.documentId, { roundId: thread.roundId });
      return thread.id;
    },
  }),

  deletePrismReviewThreadComment: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: { commentId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREAD_COMMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const { comment, thread } = await ownedComment(input.commentId, ctx.session.userId);
      if (comment.author !== 'USER' || comment.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'permission_denied', status: 403 });
      }

      await db.delete(PrismReviewThreadComments).where(eq(PrismReviewThreadComments.id, comment.id));
      pubsub.publish('prism:review', thread.documentId, { roundId: thread.roundId });
      return thread.id;
    },
  }),

  closePrismReviewThread: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: { threadId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREADS) }) },
    resolve: async (_, { input }, ctx) => setThreadState(input.threadId, ctx.session.userId, 'OPEN', 'CLOSED'),
  }),

  reopenPrismReviewThread: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: { threadId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREADS) }) },
    resolve: async (_, { input }, ctx) => setThreadState(input.threadId, ctx.session.userId, 'CLOSED', 'OPEN'),
  }),

  reactPrismReviewThread: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: {
      threadId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREADS) }),
      value: t.input.field({ type: PrismReaction, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const thread = await ownedThread(input.threadId, ctx.session.userId);
      await db
        .update(PrismReviewThreads)
        .set({ reaction: input.value ?? null })
        .where(eq(PrismReviewThreads.id, thread.id));
      pubsub.publish('prism:review', thread.documentId, { roundId: thread.roundId });
      return thread.id;
    },
  }),
}));

const ownedThread = async (threadId: string, userId: string) => {
  const thread = await db.select().from(PrismReviewThreads).where(eq(PrismReviewThreads.id, threadId)).then(first);
  if (!thread) {
    throw new TypieError({ code: 'not_found', status: 404 });
  }

  await assertDocumentPermission({ userId, documentId: thread.documentId });
  return thread;
};

const ownedComment = async (commentId: string, userId: string) => {
  const comment = await db.select().from(PrismReviewThreadComments).where(eq(PrismReviewThreadComments.id, commentId)).then(first);
  if (!comment) {
    throw new TypieError({ code: 'not_found', status: 404 });
  }

  const thread = await ownedThread(comment.threadId, userId);
  return { comment, thread };
};

const setThreadState = async (threadId: string, userId: string, from: 'OPEN' | 'CLOSED', to: 'OPEN' | 'CLOSED'): Promise<string> => {
  const thread = await ownedThread(threadId, userId);
  if (thread.state !== from) {
    throw new TypieError({ code: 'invalid_state', status: 409 });
  }

  await db.update(PrismReviewThreads).set({ state: to, stateChangedAt: dayjs() }).where(eq(PrismReviewThreads.id, thread.id));
  pubsub.publish('prism:review', thread.documentId, { roundId: thread.roundId });
  return thread.id;
};

builder.subscriptionFields((t) => ({
  prismReviewStream: t.withAuth({ session: true }).field({
    type: PrismReviewRound,
    args: { documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    subscribe: async (_, args, ctx) => {
      await assertDocumentPermission({ userId: ctx.session.userId, documentId: args.documentId });
      return pubsub.subscribe('prism:review', args.documentId);
    },
    resolve: (event, _, ctx) => {
      // 구독 ctx는 소켓 수명이다 — 로더와 요청 메모를 비우지 않으면 두 번째 이벤트부터 첫 값이 굳는다
      clearLoaders(ctx);
      clearRoundMemos(ctx);
      return event.roundId;
    },
  }),
}));

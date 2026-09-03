import {
  PrismReaction,
  PrismReviewCommentAuthor,
  PrismReviewPass,
  PrismReviewRoundState,
  PrismReviewThreadState,
  PrismReviewTier,
} from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { anchorQuote } from '@typie/prism';
import dayjs from 'dayjs';
import { and, asc, count, desc, eq, isNotNull, lt, ne, sql } from 'drizzle-orm';
import { clearLoaders } from '#/context.ts';
import {
  db,
  first,
  firstOrThrow,
  PrismCreditEntries,
  PrismReviewLineages,
  PrismReviewRounds,
  PrismReviewThreadComments,
  PrismReviewThreads,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { assertDocumentPermission } from '#/utils/permission.ts';
import { assertPrismAccess } from '#/utils/prism-access.ts';
import { toDisplayCredits } from '#/utils/prism-credit-core.ts';
import { prepareReviewSnapshot } from '#/utils/prism-review.ts';
import {
  detailOutcome,
  dispositionSummary,
  hasDetail,
  lineageLocked,
  roundState,
  summarizeOutcome,
  threadIsNew,
} from '#/utils/prism-review-core.ts';
import {
  clearRoundMemos,
  ensureRoundThreads,
  lineageRounds,
  preloadThreadComments,
  publishThread,
  recordView,
  roundSeats,
  threadComments,
  viewedRoundOf,
  viewSeat,
} from '#/utils/prism-review-threads.ts';
import { builder } from '../builder.ts';
import {
  Document,
  PrismReviewLineage,
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

// 리뷰 시점에 캡처한 StableSelection — 코멘트 selection과 같은 형식. 자리를 못 찾은 앵커는 null
const PrismReviewAnchor = builder.simpleObject('PrismReviewAnchor', {
  fields: (t) => ({ selection: t.field({ type: 'JSON', nullable: true }) }),
});

const PrismReviewStrength = builder.simpleObject('PrismReviewStrength', {
  fields: (t) => ({
    quote: t.string(),
    body: t.string({ nullable: true }),
    anchors: t.field({ type: [PrismReviewAnchor] }),
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

const PrismReviewDispositionSummary = builder.simpleObject('PrismReviewDispositionSummary', {
  fields: (t) => ({ carried: t.int(), resolved: t.int(), withdrawn: t.int(), new: t.int() }),
});

const PrismReviewSnapshot = builder.simpleObject('PrismReviewSnapshot', {
  fields: (t) => ({ versionId: t.id(), characterCount: t.int() }),
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

// 결과가 없거나 거부된 회차는 계보의 회차로 세지 않는다 — 작가에게 보여 줄 리뷰가 없다
const completedRounds = (rounds: Awaited<ReturnType<typeof lineageRounds>>) =>
  rounds.filter(
    (round) => round.workflowState === 'COMPLETED' && round.result !== null && summarizeOutcome(round.result).rejection === null,
  );

PrismReviewLineage.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    tier: t.expose('tier', { type: PrismReviewTier }),
    // 첫 회차가 아직 도는 계보에는 보여 줄 회차가 없다 — PrismReviewRound.lineage로도 닿으므로 null을 낸다
    latestRound: t.field({
      type: PrismReviewRound,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const [latest] = completedRounds(await lineageRounds(ctx, self.id));
        return latest?.id ?? null;
      },
    }),
    roundCount: t.int({ resolve: async (self, _, ctx) => completedRounds(await lineageRounds(ctx, self.id)).length }),
    locked: t.boolean({ resolve: async (self, _, ctx) => lineageLocked(await lineageRounds(ctx, self.id)) }),
  }),
});

PrismReviewThread.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    issueIndex: t.int({
      args: { roundId: t.arg.id({ required: false, validate: validateDbId(TableCode.PRISM_REVIEW_ROUNDS) }) },
      resolve: async (self, args, ctx) => {
        const seat = await viewSeat(ctx, self.id, args.roundId ?? undefined);
        return seat?.issueIndex ?? 0;
      },
    }),
    issueId: t.exposeString('issueId', { nullable: true }),
    trait: t.exposeString('trait'),
    pass: t.expose('pass', { type: PrismReviewPass }),
    body: t.exposeString('body', { nullable: true }),
    anchors: t.field({
      type: [PrismReviewAnchor],
      args: { roundId: t.arg.id({ required: false, validate: validateDbId(TableCode.PRISM_REVIEW_ROUNDS) }) },
      resolve: async (self, args, ctx) => {
        const seat = await viewSeat(ctx, self.id, args.roundId ?? undefined);
        return seat?.anchors ?? [];
      },
    }),
    quote: t.string({
      args: { roundId: t.arg.id({ required: false, validate: validateDbId(TableCode.PRISM_REVIEW_ROUNDS) }) },
      // 카드는 앵커가 살아 있든 자리를 잃었든 이 인용을 보여 준다
      resolve: async (self, args, ctx) => {
        const seat = await viewSeat(ctx, self.id, args.roundId ?? undefined);
        if (seat === null) {
          return '';
        }

        return anchorQuote(seat.anchors);
      },
    }),
    state: t.expose('state', { type: PrismReviewThreadState }),
    stateChangedAt: t.expose('stateChangedAt', { type: 'DateTime', nullable: true }),
    reaction: t.expose('reaction', { type: PrismReaction, nullable: true }),
    comments: t.field({ type: [PrismReviewThreadComment], resolve: (self, _, ctx) => threadComments(ctx, self.id) }),
    lineage: t.field({ type: PrismReviewLineage, resolve: (self) => self.lineageId }),
    settledRound: t.field({ type: PrismReviewRound, nullable: true, resolve: (self) => self.settledRoundId }),
    isNew: t.boolean({
      args: { roundId: t.arg.id({ required: false, validate: validateDbId(TableCode.PRISM_REVIEW_ROUNDS) }) },
      // 어느 회차의 눈으로 보는지 기록이 없으면(뮤테이션 반환 등) 표지를 세우지 않는다
      resolve: async (self, args, ctx) => {
        const viewRoundId = args.roundId ?? viewedRoundOf(ctx, self.id);
        if (viewRoundId === null) {
          return false;
        }

        return threadIsNew(self.bornRoundId, viewRoundId, completedRounds(await lineageRounds(ctx, self.lineageId)).length);
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
              eq(PrismReviewRounds.lineageId, self.lineageId),
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
    credits: t.int({
      nullable: true,
      resolve: async (self) => {
        const charge = await db
          .select({ paidDelta: PrismCreditEntries.paidDelta, freeDelta: PrismCreditEntries.freeDelta })
          .from(PrismCreditEntries)
          .where(and(eq(PrismCreditEntries.kind, 'REVIEW_CHARGE'), eq(PrismCreditEntries.key, self.id)))
          .then(first);
        return charge ? toDisplayCredits(0 - (charge.paidDelta + charge.freeDelta)) : null;
      },
    }),
    state: t.field({
      type: PrismReviewRoundState,
      resolve: async (self, _, ctx) => {
        const workflow = self.workflowId === null ? null : await PrismWorkflow.getDataloader(ctx).load(self.workflowId);
        return roundState(self, workflow);
      },
    }),
    workflow: t.field({ type: PrismWorkflow, nullable: true, resolve: (self) => self.workflowId }),
    // 세션이 지워진 라운드는 null이다 — 프론트는 이때 "대화 보기" 문을 세우지 않는다
    sessionId: t.exposeID('sessionId', { nullable: true }),
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
      resolve: (self) => (hasDetail(self.result) ? detailOutcome(self.result, self.conclusionAnchors) : null),
    }),
    reaction: t.expose('reaction', { type: PrismReaction, nullable: true }),
    reactionNote: t.exposeString('reactionNote', { nullable: true }),
    lineage: t.field({ type: PrismReviewLineage, resolve: (self) => self.lineageId }),
    baseRound: t.field({ type: PrismReviewRound, nullable: true, resolve: (self) => self.baseRoundId }),
    dispositionSummary: t.field({
      type: PrismReviewDispositionSummary,
      nullable: true,
      resolve: (self) => dispositionSummary(self.result),
    }),
    // 이 회차가 해소·철회로 사영한 스레드 — 여백은 이걸 "정리됨" 갈래로 세운다
    settledThreads: t.field({
      type: [PrismReviewThread],
      resolve: async (self, _, ctx) => {
        const threads = await db
          .select({ id: PrismReviewThreads.id })
          .from(PrismReviewThreads)
          .where(eq(PrismReviewThreads.settledRoundId, self.id))
          .orderBy(asc(PrismReviewThreads.stateChangedAt));

        const ids = threads.map((thread) => thread.id);
        await preloadThreadComments(ctx, ids);
        return ids;
      },
    }),
    threads: t.field({
      type: [PrismReviewThread],
      resolve: async (self, _, ctx) => {
        // 배포 전에 끝난 회차는 결과만 있고 행이 없다 — 첫 조회가 메운다
        await ensureRoundThreads(self.id);
        const seats = await roundSeats(ctx, self.id);
        const ids = [...seats.entries()].toSorted((a, b) => a[1].issueIndex - b[1].issueIndex).map(([id]) => id);
        recordView(ctx, self.id, ids);
        await preloadThreadComments(ctx, ids);
        return ids;
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

  prismReviewLineages: t.field({
    type: [PrismReviewLineage],
    // 회차와 같은 눈 — 문서를 가진 사람에게만. 최근 회차가 앞에 온다
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

      const lineages = await db
        .select({ id: PrismReviewLineages.id })
        .from(PrismReviewLineages)
        .where(eq(PrismReviewLineages.documentId, self.id));

      const ranked: { id: string; latest: number }[] = [];
      for (const lineage of lineages) {
        const [latest] = completedRounds(await lineageRounds(ctx, lineage.id));
        if (latest !== undefined) {
          ranked.push({ id: lineage.id, latest: latest.round });
        }
      }

      return ranked.toSorted((a, b) => b.latest - a.latest).map((lineage) => lineage.id);
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
  preparePrismReview: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewSnapshot,
    input: { documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });
      return await prepareReviewSnapshot({ userId: ctx.session.userId, documentId: input.documentId });
    },
  }),

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
      await assertLineageUnlocked(ctx, thread.lineageId);
      const body = input.body.trim();
      if (body.length === 0) {
        throw new TypieError({ code: 'invalid_input', status: 400 });
      }

      await db.insert(PrismReviewThreadComments).values({ threadId: thread.id, author: 'USER', userId: ctx.session.userId, body });
      await publishThread(thread);
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
      await assertLineageUnlocked(ctx, thread.lineageId);
      if (comment.author !== 'USER' || comment.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'permission_denied', status: 403 });
      }

      const body = input.body.trim();
      if (body.length === 0) {
        throw new TypieError({ code: 'invalid_input', status: 400 });
      }

      await db.update(PrismReviewThreadComments).set({ body }).where(eq(PrismReviewThreadComments.id, comment.id));
      await publishThread(thread);
      return thread.id;
    },
  }),

  deletePrismReviewThreadComment: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: { commentId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREAD_COMMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const { comment, thread } = await ownedComment(input.commentId, ctx.session.userId);
      await assertLineageUnlocked(ctx, thread.lineageId);
      if (comment.author !== 'USER' || comment.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'permission_denied', status: 403 });
      }

      await db.delete(PrismReviewThreadComments).where(eq(PrismReviewThreadComments.id, comment.id));
      await publishThread(thread);
      return thread.id;
    },
  }),

  closePrismReviewThread: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: { threadId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREADS) }) },
    resolve: async (_, { input }, ctx) => setThreadState(ctx, input.threadId, ctx.session.userId, 'OPEN', 'CLOSED'),
  }),

  reopenPrismReviewThread: t.withAuth({ session: true }).fieldWithInput({
    type: PrismReviewThread,
    input: { threadId: t.input.id({ validate: validateDbId(TableCode.PRISM_REVIEW_THREADS) }) },
    resolve: async (_, { input }, ctx) => setThreadState(ctx, input.threadId, ctx.session.userId, 'CLOSED', 'OPEN'),
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
      await publishThread(thread);
      return thread.id;
    },
  }),
}));

// 리뷰가 도는 동안 계보의 스레드는 읽기 전용이다 — 사영이 덮어쓸 상태를 작가가 함께 건드리지 못하게
const assertLineageUnlocked = async (scope: object, lineageId: string): Promise<void> => {
  if (lineageLocked(await lineageRounds(scope, lineageId))) {
    throw new TypieError({ code: 'prism_review_running', status: 409 });
  }
};

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

const setThreadState = async (
  scope: object,
  threadId: string,
  userId: string,
  from: 'OPEN' | 'CLOSED',
  to: 'OPEN' | 'CLOSED',
): Promise<string> => {
  const thread = await ownedThread(threadId, userId);
  await assertLineageUnlocked(scope, thread.lineageId);
  if (thread.state !== from) {
    throw new TypieError({ code: 'invalid_state', status: 409 });
  }

  await db.update(PrismReviewThreads).set({ state: to, stateChangedAt: dayjs() }).where(eq(PrismReviewThreads.id, thread.id));
  await publishThread(thread);
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

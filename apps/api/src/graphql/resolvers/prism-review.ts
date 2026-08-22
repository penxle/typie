import { PrismReaction, PrismReviewRoundState, PrismReviewTier } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, asc, count, eq, isNotNull, lt, ne, sql } from 'drizzle-orm';
import { db, first, firstOrThrow, PrismReviewDocumentVersions, PrismReviewRounds, TableCode, validateDbId } from '#/db/index.ts';
import { assertDocumentPermission } from '#/utils/permission.ts';
import { detailOutcome, hasDetail, roundState, summarizeOutcome } from '#/utils/prism-review-core.ts';
import { builder } from '../builder.ts';
import { Document, PrismReviewRound, PrismSession, PrismWorkflow } from '../objects.ts';

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

const PrismReviewStrength = builder.simpleObject('PrismReviewStrength', {
  fields: (t) => ({ quote: t.string(), body: t.string({ nullable: true }) }),
});

const PrismReviewVerdict = builder.simpleObject('PrismReviewVerdict', {
  fields: (t) => ({ trait: t.string(), label: t.string({ nullable: true }), note: t.string({ nullable: true }) }),
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
      resolve: async (self) => {
        if (!hasDetail(self.result)) return null;

        const version = await db
          .select({ content: PrismReviewDocumentVersions.content })
          .from(PrismReviewDocumentVersions)
          .where(eq(PrismReviewDocumentVersions.id, self.documentVersionId))
          .then(firstOrThrow);

        return detailOutcome(self.result, version.content);
      },
    }),
    reaction: t.expose('reaction', { type: PrismReaction, nullable: true }),
    reactionNote: t.exposeString('reactionNote', { nullable: true }),
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
}));

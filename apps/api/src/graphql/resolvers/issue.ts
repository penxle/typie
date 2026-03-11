import dayjs from 'dayjs';
import { and, desc, eq, inArray } from 'drizzle-orm';
import { db, Entities, firstOrThrow, IssueEntities, Issues, TableCode, validateDbId } from '@/db';
import { EntityState, IssuePriority, IssueState, IssueStatus } from '@/enums';
import { TypieError } from '@/errors';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, Issue, isTypeOf, Site } from '../objects';

Issue.implement({
  isTypeOf: isTypeOf(TableCode.ISSUES),
  fields: (t) => ({
    id: t.exposeID('id'),
    content: t.exposeString('content'),
    status: t.expose('status', { type: IssueStatus }),
    priority: t.expose('priority', { type: IssuePriority }),
    dueAt: t.expose('dueAt', { type: 'DateTime', nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

    site: t.expose('siteId', { type: Site }),

    entities: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Issue.entities',
          load: (issueIds: string[]) =>
            db
              .select({ issueId: IssueEntities.issueId, entityId: IssueEntities.entityId })
              .from(IssueEntities)
              .innerJoin(Entities, eq(IssueEntities.entityId, Entities.id))
              .where(and(inArray(IssueEntities.issueId, issueIds), eq(Entities.state, EntityState.ACTIVE))),
          key: ({ issueId }) => issueId,
          many: true,
        });

        const rows = await loader.load(self.id);
        // eslint-disable-next-line @typescript-eslint/no-explicit-any -- loadable ref resolves from IDs
        return rows.map((r) => r.entityId) as any;
      },
    }),
  }),
});

const issuePriorityOrder = { URGENT: 0, HIGH: 1, MEDIUM: 2, LOW: 3, NONE: 4 } as const;

builder.queryFields((t) => ({
  issues: t.withAuth({ session: true }).field({
    type: [Issue],
    args: {
      siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }),
      status: t.arg({ type: IssueStatus, required: false }),
      priority: t.arg({ type: IssuePriority, required: false }),
    },
    resolve: async (_, args, ctx) => {
      await assertSitePermission({ userId: ctx.session.userId, siteId: args.siteId });

      const conditions = [eq(Issues.siteId, args.siteId), eq(Issues.state, IssueState.ACTIVE)];

      if (args.status) {
        conditions.push(eq(Issues.status, args.status));
      }

      if (args.priority) {
        conditions.push(eq(Issues.priority, args.priority));
      }

      const issues = await db
        .select()
        .from(Issues)
        .where(and(...conditions))
        .orderBy(desc(Issues.priority), desc(Issues.createdAt));

      return issues.toSorted((a, b) => {
        const pa = issuePriorityOrder[a.priority as keyof typeof issuePriorityOrder] ?? 4;
        const pb = issuePriorityOrder[b.priority as keyof typeof issuePriorityOrder] ?? 4;
        if (pa !== pb) return pa - pb;
        return b.createdAt.valueOf() - a.createdAt.valueOf();
      });
    },
  }),

  issue: t.withAuth({ session: true }).field({
    type: Issue,
    args: {
      issueId: t.arg.id({ validate: validateDbId(TableCode.ISSUES) }),
    },
    resolve: async (_, args, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, args.issueId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: issue.siteId });

      return issue;
    },
  }),
}));

builder.mutationFields((t) => ({
  createIssue: t.withAuth({ session: true }).fieldWithInput({
    type: Issue,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      content: t.input.string(),
      status: t.input.field({ type: IssueStatus, required: false }),
      priority: t.input.field({ type: IssuePriority, required: false }),
      dueAt: t.input.field({ type: 'DateTime', required: false }),
      entityIds: t.input.idList({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({ userId: ctx.session.userId, siteId: input.siteId });

      if (input.entityIds?.length) {
        const entities = await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(inArray(Entities.id, input.entityIds), eq(Entities.siteId, input.siteId), eq(Entities.state, EntityState.ACTIVE)));

        if (entities.length !== input.entityIds.length) {
          throw new TypieError({ code: 'not_found' });
        }
      }

      return await db.transaction(async (tx) => {
        const issue = await tx
          .insert(Issues)
          .values({
            siteId: input.siteId,
            content: input.content,
            status: input.status ?? undefined,
            priority: input.priority ?? undefined,
            dueAt: input.dueAt ?? undefined,
          })
          .returning()
          .then(firstOrThrow);

        if (input.entityIds?.length) {
          await tx.insert(IssueEntities).values(
            input.entityIds.map((entityId) => ({
              issueId: issue.id,
              entityId,
            })),
          );
        }

        return issue;
      });
    },
  }),

  updateIssue: t.withAuth({ session: true }).fieldWithInput({
    type: Issue,
    input: {
      issueId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
      content: t.input.string({ required: false }),
      status: t.input.field({ type: IssueStatus, required: false }),
      priority: t.input.field({ type: IssuePriority, required: false }),
      dueAt: t.input.field({ type: 'DateTime', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const existing = await db
        .select({ siteId: Issues.siteId })
        .from(Issues)
        .where(and(eq(Issues.id, input.issueId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: existing.siteId });

      return await db
        .update(Issues)
        .set({
          content: input.content ?? undefined,
          status: input.status ?? undefined,
          priority: input.priority ?? undefined,
          ...(input.dueAt === undefined ? {} : { dueAt: input.dueAt }),
          updatedAt: dayjs(),
        })
        .where(eq(Issues.id, input.issueId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  deleteIssue: t.withAuth({ session: true }).fieldWithInput({
    type: Issue,
    input: {
      issueId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const existing = await db
        .select({ siteId: Issues.siteId })
        .from(Issues)
        .where(and(eq(Issues.id, input.issueId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: existing.siteId });

      return await db.update(Issues).set({ state: IssueState.DELETED }).where(eq(Issues.id, input.issueId)).returning().then(firstOrThrow);
    },
  }),

  addIssueEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Issue,
    input: {
      issueId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, input.issueId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: issue.siteId });

      await db
        .select({ id: Entities.id })
        .from(Entities)
        .where(and(eq(Entities.id, input.entityId), eq(Entities.siteId, issue.siteId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);

      await db.insert(IssueEntities).values({ issueId: input.issueId, entityId: input.entityId }).onConflictDoNothing();

      return issue;
    },
  }),

  removeIssueEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Issue,
    input: {
      issueId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, input.issueId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: issue.siteId });

      await db.delete(IssueEntities).where(and(eq(IssueEntities.issueId, input.issueId), eq(IssueEntities.entityId, input.entityId)));

      return issue;
    },
  }),
}));

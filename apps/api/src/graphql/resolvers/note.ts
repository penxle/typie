import dayjs from 'dayjs';
import { and, asc, eq, inArray } from 'drizzle-orm';
import { db, Entities, first, firstOrThrow, IssueEntities, Issues, Sites, TableCode, validateDbId } from '@/db';
import { EntityState, IssueState } from '@/enums';
import { builder } from '../builder';
import { Note } from '../objects';

const priorityRank: Record<string, number> = { NONE: 0, LOW: 1, MEDIUM: 2, HIGH: 3, URGENT: 4 };

const toNoteShape = (issue: typeof Issues.$inferSelect, userId: string, entityId: string | null) => ({
  id: issue.id,
  user: userId,
  entity: entityId,
  content: issue.content,
  color: 'gray',
  order: `${9 - (priorityRank[issue.priority] ?? 0)}_${issue.createdAt.toISOString()}`,
  createdAt: issue.createdAt,
  updatedAt: issue.updatedAt,
});

builder.queryFields((t) => ({
  notes: t.withAuth({ session: true }).field({
    type: [Note],
    args: {
      entityId: t.arg.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      siteId: t.arg.id({ required: false, validate: validateDbId(TableCode.SITES) }),
    },
    resolve: async (_, args, ctx) => {
      let siteId: string | undefined;

      if (args.siteId) {
        // Verify the user owns this site
        const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, args.siteId)).then(firstOrThrow);

        if (site.userId !== ctx.session.userId) {
          return [];
        }

        siteId = args.siteId;
      } else {
        const fallbackSite = await db
          .select({ id: Sites.id })
          .from(Sites)
          .where(eq(Sites.userId, ctx.session.userId))
          .orderBy(asc(Sites.createdAt))
          .limit(1)
          .then(first);
        siteId = fallbackSite?.id;
      }

      if (!siteId) {
        return [];
      }

      const issues = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.siteId, siteId), eq(Issues.state, IssueState.ACTIVE)));

      const issueIds = issues.map((i) => i.id);
      const entityRows = issueIds.length > 0 ? await db.select().from(IssueEntities).where(inArray(IssueEntities.issueId, issueIds)) : [];

      const firstEntityMap = new Map<string, string>();
      for (const row of entityRows) {
        if (!firstEntityMap.has(row.issueId)) {
          firstEntityMap.set(row.issueId, row.entityId);
        }
      }

      let result = issues.map((issue) => toNoteShape(issue, ctx.session.userId, firstEntityMap.get(issue.id) ?? null));

      if (args.entityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.id, args.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);

        const issueIdsForEntity = new Set(entityRows.filter((r) => r.entityId === args.entityId).map((r) => r.issueId));
        result = result.filter((n) => issueIdsForEntity.has(n.id));
      }

      return result.toSorted((a, b) => a.order.localeCompare(b.order));
    },
  }),

  note: t.withAuth({ session: true }).field({
    type: Note,
    args: {
      noteId: t.arg.id({ validate: validateDbId(TableCode.ISSUES) }),
    },
    resolve: async (_, args, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, args.noteId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, issue.siteId)).then(firstOrThrow);

      if (site.userId !== ctx.session.userId) {
        throw new Error('assertion failed');
      }

      const entityRow = await db
        .select({ entityId: IssueEntities.entityId })
        .from(IssueEntities)
        .where(eq(IssueEntities.issueId, issue.id))
        .limit(1)
        .then(first);

      return toNoteShape(issue, ctx.session.userId, entityRow?.entityId ?? null);
    },
  }),
}));

builder.mutationFields((t) => ({
  createNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      entityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      content: t.input.string(),
      color: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const site = await db
        .select({ id: Sites.id })
        .from(Sites)
        .where(eq(Sites.userId, ctx.session.userId))
        .orderBy(asc(Sites.createdAt))
        .limit(1)
        .then(firstOrThrow);

      let validEntityId: string | null = null;

      if (input.entityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.id, input.entityId), eq(Entities.siteId, site.id), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);
        validEntityId = input.entityId;
      }

      return await db.transaction(async (tx) => {
        const issue = await tx
          .insert(Issues)
          .values({
            siteId: site.id,
            content: input.content,
          })
          .returning()
          .then(firstOrThrow);

        if (validEntityId) {
          await tx.insert(IssueEntities).values({
            issueId: issue.id,
            entityId: validEntityId,
          });
        }

        return toNoteShape(issue, ctx.session.userId, validEntityId);
      });
    },
  }),

  updateNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
      entityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      content: t.input.string({ required: false }),
      color: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, input.noteId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, issue.siteId)).then(firstOrThrow);

      if (site.userId !== ctx.session.userId) {
        throw new Error('assertion failed');
      }

      return await db.transaction(async (tx) => {
        const updated =
          input.content == null
            ? issue
            : await tx
                .update(Issues)
                .set({ content: input.content, updatedAt: dayjs() })
                .where(eq(Issues.id, input.noteId))
                .returning()
                .then(firstOrThrow);

        if (input.entityId) {
          await tx
            .select({ id: Entities.id })
            .from(Entities)
            .where(and(eq(Entities.id, input.entityId), eq(Entities.siteId, issue.siteId), eq(Entities.state, EntityState.ACTIVE)))
            .then(firstOrThrow);

          await tx.delete(IssueEntities).where(eq(IssueEntities.issueId, input.noteId));
          await tx.insert(IssueEntities).values({ issueId: input.noteId, entityId: input.entityId });
        }

        const entityRow = await tx
          .select({ entityId: IssueEntities.entityId })
          .from(IssueEntities)
          .where(eq(IssueEntities.issueId, input.noteId))
          .limit(1)
          .then(first);

        return toNoteShape(updated, ctx.session.userId, entityRow?.entityId ?? null);
      });
    },
  }),

  moveNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, input.noteId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, issue.siteId)).then(firstOrThrow);

      if (site.userId !== ctx.session.userId) {
        throw new Error('assertion failed');
      }

      const entityRow = await db
        .select({ entityId: IssueEntities.entityId })
        .from(IssueEntities)
        .where(eq(IssueEntities.issueId, issue.id))
        .limit(1)
        .then(first);

      return toNoteShape(issue, ctx.session.userId, entityRow?.entityId ?? null);
    },
  }),

  deleteNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.ISSUES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const issue = await db
        .select()
        .from(Issues)
        .where(and(eq(Issues.id, input.noteId), eq(Issues.state, IssueState.ACTIVE)))
        .then(firstOrThrow);

      const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, issue.siteId)).then(firstOrThrow);

      if (site.userId !== ctx.session.userId) {
        throw new Error('assertion failed');
      }

      const deleted = await db
        .update(Issues)
        .set({ state: IssueState.DELETED })
        .where(eq(Issues.id, input.noteId))
        .returning()
        .then(firstOrThrow);

      const entityRow = await db
        .select({ entityId: IssueEntities.entityId })
        .from(IssueEntities)
        .where(eq(IssueEntities.issueId, input.noteId))
        .limit(1)
        .then(first);

      return toNoteShape(deleted, ctx.session.userId, entityRow?.entityId ?? null);
    },
  }),
}));

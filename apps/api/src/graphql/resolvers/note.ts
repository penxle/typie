import dayjs from 'dayjs';
import { and, asc, desc, eq, inArray } from 'drizzle-orm';
import { db, Entities, first, firstOrThrow, NoteEntities, Notes, Sites, TableCode, validateDbId } from '@/db';
import { EntityState, NoteState, NoteStatus } from '@/enums';
import { TypieError } from '@/errors';
import { generateFractionalOrder } from '@/utils/order';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, isTypeOf, Note, Site, User } from '../objects';

Note.implement({
  isTypeOf: isTypeOf(TableCode.NOTES),
  fields: (t) => ({
    id: t.exposeID('id'),
    content: t.exposeString('content'),
    color: t.exposeString('color'),
    order: t.exposeString('order'),
    status: t.expose('status', { type: NoteStatus }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

    user: t.expose('userId', { type: User }),
    site: t.expose('siteId', { type: Site, nullable: true }),

    entity: t.field({
      type: Entity,
      nullable: true,
      resolve: async (self) => {
        const row = await db
          .select({ entityId: NoteEntities.entityId })
          .from(NoteEntities)
          .where(eq(NoteEntities.noteId, self.id))
          .limit(1)
          .then(first);
        // eslint-disable-next-line @typescript-eslint/no-explicit-any -- loadable ref resolves from IDs
        return (row?.entityId ?? null) as any;
      },
    }),

    entities: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Note.entities',
          load: (noteIds: string[]) =>
            db
              .select({ noteId: NoteEntities.noteId, entityId: NoteEntities.entityId })
              .from(NoteEntities)
              .innerJoin(Entities, eq(NoteEntities.entityId, Entities.id))
              .where(and(inArray(NoteEntities.noteId, noteIds), eq(Entities.state, EntityState.ACTIVE))),
          key: ({ noteId }) => noteId,
          many: true,
        });

        const rows = await loader.load(self.id);
        // eslint-disable-next-line @typescript-eslint/no-explicit-any -- loadable ref resolves from IDs
        return rows.map((r) => r.entityId) as any;
      },
    }),
  }),
});

builder.queryFields((t) => ({
  notes: t.withAuth({ session: true }).field({
    type: [Note],
    args: {
      entityId: t.arg.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      siteId: t.arg.id({ required: false, validate: validateDbId(TableCode.SITES) }),
      status: t.arg({ type: NoteStatus, required: false }),
    },
    resolve: async (_, args, ctx) => {
      let siteId: string | undefined;

      if (args.siteId) {
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

      const conditions = [eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)];

      if (siteId) {
        conditions.push(eq(Notes.siteId, siteId));
      }

      if (args.status) {
        conditions.push(eq(Notes.status, args.status));
      }

      if (args.entityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.id, args.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);

        const noteIds = await db
          .select({ noteId: NoteEntities.noteId })
          .from(NoteEntities)
          .where(eq(NoteEntities.entityId, args.entityId))
          .then((rows) => rows.map((r) => r.noteId));

        if (noteIds.length === 0) {
          return [];
        }

        conditions.push(inArray(Notes.id, noteIds));
      }

      return await db
        .select()
        .from(Notes)
        .where(and(...conditions))
        .orderBy(asc(Notes.order));
    },
  }),

  note: t.withAuth({ session: true }).field({
    type: Note,
    args: {
      noteId: t.arg.id({ validate: validateDbId(TableCode.NOTES) }),
    },
    resolve: async (_, args, ctx) => {
      const note = await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      return note;
    },
  }),
}));

builder.mutationFields((t) => ({
  createNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      entityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      siteId: t.input.id({ required: false, validate: validateDbId(TableCode.SITES) }),
      entityIds: t.input.idList({ required: false }),
      content: t.input.string(),
      color: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      let siteId: string;

      if (input.siteId) {
        await assertSitePermission({ userId: ctx.session.userId, siteId: input.siteId });
        siteId = input.siteId;
      } else if (input.entityId) {
        const entity = await db
          .select({ siteId: Entities.siteId })
          .from(Entities)
          .where(and(eq(Entities.id, input.entityId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);
        await assertSitePermission({ userId: ctx.session.userId, siteId: entity.siteId });
        siteId = entity.siteId;
      } else {
        const site = await db
          .select({ id: Sites.id })
          .from(Sites)
          .where(eq(Sites.userId, ctx.session.userId))
          .orderBy(asc(Sites.createdAt))
          .limit(1)
          .then(firstOrThrow);
        siteId = site.id;
      }

      const allEntityIds = [...new Set([...(input.entityId ? [input.entityId] : []), ...(input.entityIds ?? [])])];

      if (allEntityIds.length > 0) {
        const entities = await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(inArray(Entities.id, allEntityIds), eq(Entities.siteId, siteId), eq(Entities.state, EntityState.ACTIVE)));

        if (entities.length !== allEntityIds.length) {
          throw new TypieError({ code: 'not_found' });
        }
      }

      // Get the last order for this user to place the new note at the end
      const lastNote = await db
        .select({ order: Notes.order })
        .from(Notes)
        .where(and(eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .orderBy(desc(Notes.order))
        .limit(1)
        .then(first);

      const order = generateFractionalOrder({ lower: lastNote?.order, upper: null });

      return await db.transaction(async (tx) => {
        const note = await tx
          .insert(Notes)
          .values({
            userId: ctx.session.userId,
            siteId,
            content: input.content,
            color: input.color,
            order,
          })
          .returning()
          .then(firstOrThrow);

        if (allEntityIds.length > 0) {
          await tx.insert(NoteEntities).values(
            allEntityIds.map((entityId) => ({
              noteId: note.id,
              entityId,
            })),
          );
        }

        return note;
      });
    },
  }),

  updateNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      entityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      content: t.input.string({ required: false }),
      color: t.input.string({ required: false }),
      status: t.input.field({ type: NoteStatus, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const note = await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      return await db.transaction(async (tx) => {
        const updated = await tx
          .update(Notes)
          .set({
            content: input.content ?? undefined,
            color: input.color ?? undefined,
            status: input.status ?? undefined,
            updatedAt: dayjs(),
          })
          .where(eq(Notes.id, input.noteId))
          .returning()
          .then(firstOrThrow);

        if (input.entityId) {
          await tx
            .select({ id: Entities.id })
            .from(Entities)
            .where(
              and(
                eq(Entities.id, input.entityId),
                eq(Entities.state, EntityState.ACTIVE),
                ...(note.siteId ? [eq(Entities.siteId, note.siteId)] : []),
              ),
            )
            .then(firstOrThrow);

          await tx.delete(NoteEntities).where(eq(NoteEntities.noteId, input.noteId));
          await tx.insert(NoteEntities).values({ noteId: input.noteId, entityId: input.entityId });
        }

        return updated;
      });
    },
  }),

  moveNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const note = await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      const newOrder = generateFractionalOrder({
        lower: input.lowerOrder,
        upper: input.upperOrder,
      });

      return await db
        .update(Notes)
        .set({ order: newOrder, updatedAt: dayjs() })
        .where(eq(Notes.id, note.id))
        .returning()
        .then(firstOrThrow);
    },
  }),

  deleteNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
    },
    resolve: async (_, { input }, ctx) => {
      await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      return await db
        .update(Notes)
        .set({ state: NoteState.DELETED, updatedAt: dayjs() })
        .where(eq(Notes.id, input.noteId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  addNoteEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const note = await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      await db
        .select({ id: Entities.id })
        .from(Entities)
        .where(and(eq(Entities.id, input.entityId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);

      await db.insert(NoteEntities).values({ noteId: input.noteId, entityId: input.entityId }).onConflictDoNothing();

      return note;
    },
  }),

  removeNoteEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const note = await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      await db.delete(NoteEntities).where(and(eq(NoteEntities.noteId, input.noteId), eq(NoteEntities.entityId, input.entityId)));

      return note;
    },
  }),
}));

import dayjs from 'dayjs';
import { and, asc, eq } from 'drizzle-orm';
import { db, Entities, first, firstOrThrow, Notes, TableCode, validateDbId } from '@/db';
import { EntityState, NoteState } from '@/enums';
import { generateFractionalOrder } from '@/utils';
import { builder } from '../builder';
import { Entity, isTypeOf, Note, User } from '../objects';

Note.implement({
  isTypeOf: isTypeOf(TableCode.NOTES),
  fields: (t) => ({
    id: t.exposeID('id'),
    order: t.exposeString('order'),
    content: t.exposeString('content'),
    color: t.exposeString('color'),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

    user: t.expose('userId', { type: User }),
    entity: t.expose('entityId', { type: Entity, nullable: true }),
  }),
});

builder.queryFields((t) => ({
  notes: t.withAuth({ session: true }).field({
    type: [Note],
    args: {
      entityId: t.arg.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, args, ctx) => {
      const conditions = [eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)];

      if (args.entityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.id, args.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);

        conditions.push(eq(Notes.entityId, args.entityId));
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
      return await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);
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
      if (input.entityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.id, input.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);
      }

      const firstNote = await db
        .select({ order: Notes.order })
        .from(Notes)
        .where(eq(Notes.userId, ctx.session.userId))
        .orderBy(asc(Notes.order))
        .limit(1)
        .then(first);

      return await db
        .insert(Notes)
        .values({
          userId: ctx.session.userId,
          entityId: input.entityId,
          content: input.content,
          color: input.color,
          order: generateFractionalOrder({ lower: null, upper: firstNote?.order }),
        })
        .returning()
        .then(firstOrThrow);
    },
  }),

  updateNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      entityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      content: t.input.string({ required: false }),
      color: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      await db
        .select({ id: Notes.id })
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      if (input.entityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.id, input.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);
      }

      return await db
        .update(Notes)
        .set({
          entityId: input.entityId,
          content: input.content ?? undefined,
          color: input.color ?? undefined,
          updatedAt: dayjs(),
        })
        .where(eq(Notes.id, input.noteId))
        .returning()
        .then(firstOrThrow);
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
      await db
        .select({ id: Notes.id })
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrow);

      return await db
        .update(Notes)
        .set({
          order: generateFractionalOrder({
            lower: input.lowerOrder,
            upper: input.upperOrder,
          }),
          updatedAt: dayjs(),
        })
        .where(eq(Notes.id, input.noteId))
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
      return await db
        .update(Notes)
        .set({ state: NoteState.DELETED })
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .returning()
        .then(firstOrThrow);
    },
  }),
}));

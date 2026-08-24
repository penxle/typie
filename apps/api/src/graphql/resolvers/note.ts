import { EntityState, NoteState, NoteStatus, SiteState } from '@typie/lib/enums';
import { NotFoundError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, eq, inArray } from 'drizzle-orm';
import { filter, pipe } from 'graphql-yoga';
import { db, Entities, first, firstOrThrow, firstOrThrowWith, NoteEntities, Notes, Sites, TableCode, validateDbId } from '#/db/index.ts';
import { NOTE_UPDATE_KINDS, pubsub } from '#/pubsub.ts';
import { addNoteEntityCore, createNoteCore, deleteNoteCore, removeNoteEntityCore, updateNoteCore } from '#/utils/note-actions.ts';
import { generateFractionalOrder } from '#/utils/order.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { builder } from '../builder.ts';
import { Entity, isTypeOf, Note, Site, User } from '../objects.ts';

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
    site: t.expose('siteId', { type: Site }),

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
      const entity = args.entityId
        ? await db
            .select({ siteId: Entities.siteId })
            .from(Entities)
            .where(and(eq(Entities.id, args.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
            .then(firstOrThrow)
        : null;

      let siteId: string | undefined;

      if (args.siteId) {
        const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, args.siteId)).then(firstOrThrow);

        if (site.userId !== ctx.session.userId) {
          return [];
        }

        siteId = args.siteId;
      } else if (entity) {
        siteId = entity.siteId;
      } else {
        const fallbackSite = await db
          .select({ id: Sites.id })
          .from(Sites)
          .where(and(eq(Sites.userId, ctx.session.userId), eq(Sites.state, SiteState.ACTIVE)))
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
    resolve: async (_, args, ctx) =>
      db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, ctx.session.userId)))
        .then(firstOrThrowWith(new NotFoundError())),
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
      clientId: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertActiveSubscription({ userId: ctx.session.userId });

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

      return await createNoteCore(db, {
        userId: ctx.session.userId,
        siteId,
        content: input.content,
        color: input.color,
        entityIds: allEntityIds,
        clientId: input.clientId,
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
      clientId: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) =>
      await updateNoteCore(db, {
        userId: ctx.session.userId,
        noteId: input.noteId,
        content: input.content,
        color: input.color,
        status: input.status,
        entityId: input.entityId,
        clientId: input.clientId,
      }),
  }),

  moveNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
      clientId: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const note = await db
        .select()
        .from(Notes)
        .where(and(eq(Notes.id, input.noteId), eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
        .then(firstOrThrowWith(new NotFoundError()));

      await assertActiveSubscription({ userId: ctx.session.userId });

      const updated = await db
        .update(Notes)
        .set({
          order: generateFractionalOrder({ lower: input.lowerOrder, upper: input.upperOrder }),
          updatedAt: dayjs(),
        })
        .where(and(eq(Notes.id, note.id), eq(Notes.state, NoteState.ACTIVE)))
        .returning()
        .then(firstOrThrowWith(new NotFoundError()));

      pubsub.publish('note:update', updated.siteId, {
        kind: 'UPDATED',
        noteId: updated.id,
        originClientId: input.clientId ?? undefined,
      });
      return updated;
    },
  }),

  deleteNote: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      clientId: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) =>
      await deleteNoteCore(db, {
        userId: ctx.session.userId,
        noteId: input.noteId,
        clientId: input.clientId,
      }),
  }),

  addNoteEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      clientId: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) =>
      await addNoteEntityCore(db, {
        userId: ctx.session.userId,
        noteId: input.noteId,
        entityId: input.entityId,
        clientId: input.clientId,
      }),
  }),

  removeNoteEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Note,
    input: {
      noteId: t.input.id({ validate: validateDbId(TableCode.NOTES) }),
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      clientId: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) =>
      await removeNoteEntityCore(db, {
        userId: ctx.session.userId,
        noteId: input.noteId,
        entityId: input.entityId,
        clientId: input.clientId,
      }),
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  noteUpdateStream: t.withAuth({ session: true }).field({
    type: t.builder.simpleObject('NoteUpdateStreamPayload', {
      fields: (t) => ({
        kind: t.field({
          type: t.builder.enumType('NoteUpdateKind', {
            values: NOTE_UPDATE_KINDS,
          }),
        }),
        noteId: t.id(),
      }),
    }),
    args: {
      siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }),
      clientId: t.arg.string(),
    },
    subscribe: async (_, args, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: args.siteId,
      });

      return pipe(
        pubsub.subscribe('note:update', args.siteId),
        filter((event) => event.originClientId !== args.clientId),
      );
    },
    resolve: (event) => event,
  }),
}));

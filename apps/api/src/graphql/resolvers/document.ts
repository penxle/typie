import dayjs from 'dayjs';
import { and, desc, eq, isNull } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, first, firstOrThrow, firstOrThrowWith, Notes, TableCode, validateDbId } from '@/db';
import { EntityAvailability, EntityState, EntityType, NoteState } from '@/enums';
import { NotFoundError } from '@/errors';
import { pubsub } from '@/pubsub';
import { generateFractionalOrder, generatePermalink, generateSlug } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Document, DocumentView, Entity, EntityView, IDocument, isTypeOf } from '../objects';

IDocument.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.string({ resolve: (self) => self.title || '(제목 없음)' }),
    nullableTitle: t.exposeString('title', { nullable: true }),
    subtitle: t.exposeString('subtitle', { nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

Document.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENTS),
  interfaces: [IDocument],
  fields: (t) => ({
    view: t.expose('id', { type: DocumentView }),

    snapshot: t.field({
      type: 'Binary',
      nullable: true,
      resolve: async (self) => {
        const content = await db
          .select({ snapshot: DocumentContents.snapshot })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(first);

        return content?.snapshot ?? null;
      },
    }),

    entity: t.expose('entityId', { type: Entity }),
  }),
});

DocumentView.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENTS),
  interfaces: [IDocument],
  fields: (t) => ({
    entity: t.expose('entityId', { type: EntityView }),
  }),
});

builder.queryFields((t) => ({
  document: t.withAuth({ session: true }).field({
    type: Document,
    args: { slug: t.arg.string() },
    resolve: async (_, args, ctx) => {
      const { document, entity } = await db
        .select({ document: Documents, entity: { siteId: Entities.siteId, availability: Entities.availability } })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Entities.slug, args.slug))
        .then(firstOrThrowWith(new NotFoundError()));

      if (entity.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: entity.siteId,
        }).catch(() => {
          throw new NotFoundError();
        });
      }

      return document;
    },
  }),
}));

builder.mutationFields((t) => ({
  createDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      let depth = 0;
      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ id: Entities.id, depth: Entities.depth })
          .from(Entities)
          .where(
            and(
              eq(Entities.siteId, input.siteId),
              eq(Entities.id, input.parentEntityId),
              eq(Entities.type, EntityType.FOLDER),
              eq(Entities.state, EntityState.ACTIVE),
            ),
          )
          .then(firstOrThrow);

        depth = parentEntity.depth + 1;
      }

      const last = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, input.siteId),
            input.parentEntityId ? eq(Entities.parentId, input.parentEntityId) : isNull(Entities.parentId),
          ),
        )
        .orderBy(desc(Entities.order))
        .limit(1)
        .then(first);

      const document = await db.transaction(async (tx) => {
        const entity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: input.siteId,
            parentId: input.parentEntityId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.DOCUMENT,
            order: generateFractionalOrder({ lower: last?.order, upper: null }),
            depth,
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const document = await tx
          .insert(Documents)
          .values({
            entityId: entity.id,
            title: null,
          })
          .returning()
          .then(firstOrThrow);

        return document;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', input.siteId, null);

      return document;
    },
  }),

  saveDocumentSnapshot: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      snapshot: t.input.field({ type: 'Binary' }),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({ siteId: Entities.siteId, availability: Entities.availability })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      if (document.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: document.siteId,
        });
      }

      const existing = await db
        .select({ id: DocumentContents.id })
        .from(DocumentContents)
        .where(eq(DocumentContents.documentId, input.documentId))
        .then(first);

      if (existing) {
        await db
          .update(DocumentContents)
          .set({ snapshot: input.snapshot, updatedAt: dayjs() })
          .where(eq(DocumentContents.documentId, input.documentId));
      } else {
        await db.insert(DocumentContents).values({
          documentId: input.documentId,
          snapshot: input.snapshot,
        });
      }

      await db.update(Documents).set({ updatedAt: dayjs() }).where(eq(Documents.id, input.documentId));

      return true;
    },
  }),

  deleteDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: { documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ id: Entities.id, siteId: Entities.siteId })
        .from(Entities)
        .innerJoin(Documents, eq(Entities.id, Documents.entityId))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({
            state: EntityState.DELETED,
            deletedAt: dayjs(),
          })
          .where(eq(Entities.id, entity.id));

        await tx
          .update(Notes)
          .set({ state: NoteState.DELETED_CASCADED })
          .where(and(eq(Notes.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)));
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.id });
      pubsub.publish('site:usage:update', entity.siteId, null);

      return input.documentId;
    },
  }),

  updateDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      title: t.input.string({ required: false }),
      subtitle: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({ entityId: Documents.entityId, siteId: Entities.siteId, availability: Entities.availability })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      if (document.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: document.siteId,
        });
      }

      const updatedDocument = await db
        .update(Documents)
        .set({
          ...(input.title !== undefined && { title: input.title }),
          ...(input.subtitle !== undefined && { subtitle: input.subtitle }),
          updatedAt: dayjs(),
        })
        .where(eq(Documents.id, input.documentId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('site:update', document.siteId, { scope: 'entity', entityId: document.entityId });

      return updatedDocument;
    },
  }),
}));

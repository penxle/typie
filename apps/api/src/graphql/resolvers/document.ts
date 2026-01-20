import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, gte, inArray, isNull, lt, sum } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { LoroDoc } from 'loro-crdt';
import { redis } from '@/cache';
import {
  db,
  DocumentCharacterCountChanges,
  DocumentContents,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  Notes,
  TableCode,
  validateDbId,
} from '@/db';
import { DocumentSyncType, DocumentType, EntityAvailability, EntityState, EntityType, EntityVisibility, NoteState } from '@/enums';
import { NotFoundError, TypieError } from '@/errors';
import * as spellcheck from '@/external/spellcheck';
import { enqueueJob } from '@/mq';
import { pubsub } from '@/pubsub';
import {
  extractLoroDocContents,
  extractLoroDocLayoutMode,
  generateFractionalOrder,
  generatePermalink,
  generateSlug,
  makeLoroDoc,
} from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { assertPlanRule } from '@/utils/plan';
import { builder } from '../builder';
import { CharacterCountChange, Document, DocumentVersion, DocumentView, Entity, EntityView, IDocument, isTypeOf } from '../objects';

IDocument.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.string({ resolve: (self) => self.title || '(제목 없음)' }),
    nullableTitle: t.exposeString('title', { nullable: true }),
    subtitle: t.exposeString('subtitle', { nullable: true }),
    type: t.expose('type', { type: DocumentType }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
    excerpt: t.string({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Document.excerpt',
          load: async (ids) => {
            return await db
              .select({ documentId: DocumentContents.documentId, text: DocumentContents.text })
              .from(DocumentContents)
              .where(inArray(DocumentContents.documentId, ids));
          },
          key: ({ documentId }) => documentId,
        });

        const content = await loader.load(self.id);
        const text = content.text.replaceAll(/\s+/g, ' ').trim();

        return text.length <= 200 ? text : text.slice(0, 200) + '...';
      },
    }),
  }),
});

Document.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENTS),
  interfaces: [IDocument],
  fields: (t) => ({
    view: t.expose('id', { type: DocumentView }),

    snapshot: t.field({
      type: 'Binary',
      resolve: async (self) => {
        const content = await db
          .select({ snapshot: DocumentContents.snapshot })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(firstOrThrow);

        return content.snapshot;
      },
    }),

    layoutMode: t.field({
      type: 'JSON',
      resolve: async (self) => {
        const content = await db
          .select({ snapshot: DocumentContents.snapshot })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(firstOrThrow);

        return extractLoroDocLayoutMode(content.snapshot);
      },
    }),

    characterCountChange: t.withAuth({ session: true }).field({
      type: CharacterCountChange,
      resolve: async (document, _, ctx) => {
        const startOfDay = dayjs().kst().startOf('day');

        const change = await db
          .select({
            additions: sum(DocumentCharacterCountChanges.additions).mapWith(Number),
            deletions: sum(DocumentCharacterCountChanges.deletions).mapWith(Number),
          })
          .from(DocumentCharacterCountChanges)
          .where(
            and(
              eq(DocumentCharacterCountChanges.userId, ctx.session.userId),
              eq(DocumentCharacterCountChanges.documentId, document.id),
              gte(DocumentCharacterCountChanges.bucket, startOfDay),
              lt(DocumentCharacterCountChanges.bucket, startOfDay.add(1, 'day')),
            ),
          )
          .then(firstOrThrow);

        return {
          date: startOfDay,
          additions: change.additions ?? 0,
          deletions: change.deletions ?? 0,
        };
      },
    }),

    entity: t.expose('entityId', { type: Entity }),

    versions: t.field({
      type: [DocumentVersion],
      args: {
        first: t.arg.int({ defaultValue: 20 }),
        before: t.arg({ type: 'DateTime', required: false }),
      },
      resolve: async (self, args) => {
        return await db
          .select()
          .from(DocumentVersions)
          .where(
            args.before
              ? and(eq(DocumentVersions.documentId, self.id), lt(DocumentVersions.createdAt, args.before))
              : eq(DocumentVersions.documentId, self.id),
          )
          .orderBy(desc(DocumentVersions.createdAt))
          .limit(args.first);
      },
    }),

    versionMetas: t.field({
      type: [
        builder.simpleObject('DocumentVersionMeta', {
          fields: (t) => ({
            id: t.id(),
            createdAt: t.field({ type: 'DateTime' }),
          }),
        }),
      ],
      resolve: async (self) => {
        return await db
          .select({ id: DocumentVersions.id, createdAt: DocumentVersions.createdAt })
          .from(DocumentVersions)
          .where(eq(DocumentVersions.documentId, self.id))
          .orderBy(asc(DocumentVersions.createdAt));
      },
    }),
  }),
});

DocumentView.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENTS),
  interfaces: [IDocument],
  fields: (t) => ({
    entity: t.expose('entityId', { type: EntityView }),

    snapshot: t.field({
      type: 'Binary',
      nullable: true,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'DocumentView.snapshot',
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentContents.documentId, snapshot: DocumentContents.snapshot })
              .from(DocumentContents)
              .where(inArray(DocumentContents.documentId, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        const content = await loader.load(self.id);
        if (!content?.snapshot) {
          return null;
        }

        const doc = new LoroDoc();
        doc.import(content.snapshot);
        return new Uint8Array(doc.export({ mode: 'shallow-snapshot', frontiers: doc.oplogFrontiers() }));
      },
    }),
  }),
});

DocumentVersion.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENT_VERSIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    version: t.field({ type: 'Binary', resolve: (self) => self.version }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
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

        const emptyDoc = makeLoroDoc();
        const snapshot = emptyDoc.export({ mode: 'snapshot' });
        const version = emptyDoc.version().encode();
        const { json, text, characterCount, blobSize } = extractLoroDocContents(emptyDoc);

        await tx.insert(DocumentContents).values({
          documentId: document.id,
          json,
          text,
          characterCount,
          blobSize,
          snapshot,
          version,
        });

        return document;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', input.siteId, null);

      await enqueueJob('document:index', document.id);

      return document;
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

      await enqueueJob('document:index', input.documentId);

      return input.documentId;
    },
  }),

  duplicateDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({
          id: Entities.id,
          siteId: Entities.siteId,
          parentEntityId: Entities.parentId,
          order: Entities.order,
          depth: Entities.depth,
        })
        .from(Entities)
        .innerJoin(Documents, eq(Entities.id, Documents.entityId))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      const nextEntity = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, entity.siteId),
            entity.parentEntityId ? eq(Entities.parentId, entity.parentEntityId) : isNull(Entities.parentId),
            gt(Entities.order, entity.order),
          ),
        )
        .orderBy(asc(Entities.order))
        .limit(1)
        .then(first);

      const document = await db
        .select({
          title: Documents.title,
          subtitle: Documents.subtitle,
          content: {
            json: DocumentContents.json,
            text: DocumentContents.text,
            characterCount: DocumentContents.characterCount,
            blobSize: DocumentContents.blobSize,
            snapshot: DocumentContents.snapshot,
            version: DocumentContents.version,
          },
        })
        .from(Documents)
        .innerJoin(DocumentContents, eq(Documents.id, DocumentContents.documentId))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertPlanRule({ userId: ctx.session.userId, rule: 'maxTotalCharacterCount' });
      await assertPlanRule({ userId: ctx.session.userId, rule: 'maxTotalBlobSize' });

      // TODO: anchors

      const notes = await db
        .select({
          content: Notes.content,
          color: Notes.color,
          order: Notes.order,
        })
        .from(Notes)
        .where(and(eq(Notes.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)))
        .orderBy(asc(Notes.order));

      let lastOrder: string | null = null;
      if (notes.length > 0) {
        const lastUserNote = await db
          .select({ order: Notes.order })
          .from(Notes)
          .where(and(eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
          .orderBy(desc(Notes.order))
          .limit(1)
          .then(first);

        lastOrder = lastUserNote?.order ?? null;
      }

      const title = `(사본) ${document.title ?? '(제목 없음)'}`;

      const newDocument = await db.transaction(async (tx) => {
        const newEntity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: entity.siteId,
            parentId: entity.parentEntityId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.DOCUMENT,
            order: generateFractionalOrder({ lower: entity.order, upper: nextEntity?.order }),
            depth: entity.depth,
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const newDocument = await tx
          .insert(Documents)
          .values({
            entityId: newEntity.id,
            title,
            subtitle: document.subtitle,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(DocumentContents).values({
          documentId: newDocument.id,
          json: document.content.json,
          text: document.content.text,
          characterCount: document.content.characterCount,
          blobSize: document.content.blobSize,
          snapshot: document.content.snapshot,
          version: document.content.version,
        });

        // TODO: anchors

        const documentVersion = await tx
          .insert(DocumentVersions)
          .values({
            documentId: newDocument.id,
            version: document.content.version,
          })
          .returning({ id: DocumentVersions.id })
          .then(firstOrThrow);

        await tx.insert(DocumentVersionContributors).values({
          versionId: documentVersion.id,
          userId: ctx.session.userId,
        });

        if (notes.length > 0) {
          const notesWithNewOrder = notes.map((note) => {
            const newOrder = generateFractionalOrder({
              lower: lastOrder,
              upper: null,
            });
            lastOrder = newOrder;

            return {
              userId: ctx.session.userId,
              entityId: newEntity.id,
              content: note.content,
              color: note.color,
              order: newOrder,
              createdAt: dayjs(),
              updatedAt: dayjs(),
            };
          });

          await tx.insert(Notes).values(notesWithNewOrder);
        }

        return newDocument;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', entity.siteId, null);

      await enqueueJob('document:index', newDocument.id);

      return newDocument;
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

      await enqueueJob('document:index', input.documentId);

      return updatedDocument;
    },
  }),

  updateDocumentType: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      type: t.input.field({ type: DocumentType }),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({ siteId: Entities.siteId, entityId: Entities.id })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: document.siteId,
      });

      const updatedDocument = await db
        .update(Documents)
        .set({ type: input.type })
        .where(eq(Documents.id, input.documentId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('site:update', document.siteId, { scope: 'site' });
      pubsub.publish('site:update', document.siteId, { scope: 'entity', entityId: document.entityId });

      return updatedDocument;
    },
  }),

  updateDocumentsOption: t.withAuth({ session: true }).fieldWithInput({
    type: [Document],
    input: {
      documentIds: t.input.idList({ validate: { items: validateDbId(TableCode.DOCUMENTS) } }),
      availability: t.input.field({ type: EntityAvailability, required: false }),
      visibility: t.input.field({ type: EntityVisibility, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const documents = await db
        .select({
          id: Documents.id,
          siteId: Entities.siteId,
          entityId: Entities.id,
        })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(and(eq(Entities.state, EntityState.ACTIVE), inArray(Documents.id, input.documentIds)));

      if (documents.length === 0) {
        throw new TypieError({ code: 'invalid_argument' });
      }

      const siteId = documents[0].siteId;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      if (documents.some((doc) => doc.siteId !== siteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      await db.transaction(async (tx) => {
        if (input.availability || input.visibility) {
          await tx
            .update(Entities)
            .set({
              availability: input.availability ?? undefined,
              visibility: input.visibility ?? undefined,
            })
            .where(
              inArray(
                Entities.id,
                documents.map((doc) => doc.entityId),
              ),
            );
        }
      });

      pubsub.publish('site:update', siteId, { scope: 'site' });

      return documents.map((doc) => doc.id);
    },
  }),

  syncDocument: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      clientId: t.input.string(),
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      type: t.input.field({ type: DocumentSyncType }),
      data: t.input.string(),
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

      if (input.type === DocumentSyncType.UPDATE) {
        pubsub.publish('document:sync', input.documentId, {
          target: `!${input.clientId}`,
          type: DocumentSyncType.UPDATE,
          data: input.data,
        });

        await redis.lpush(
          `document:sync:updates:${input.documentId}`,
          JSON.stringify({
            userId: ctx.session.userId,
            data: input.data,
          }),
        );

        await enqueueJob('document:sync:collect', input.documentId);
      } else if (input.type === DocumentSyncType.VECTOR) {
        const contents = await db
          .select({ snapshot: DocumentContents.snapshot, version: DocumentContents.version })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, input.documentId))
          .then(first);

        if (contents) {
          pubsub.publish('document:sync', input.documentId, {
            target: input.clientId,
            type: DocumentSyncType.UPDATE,
            data: contents.snapshot.toBase64(),
          });

          pubsub.publish('document:sync', input.documentId, {
            target: input.clientId,
            type: DocumentSyncType.VECTOR,
            data: contents.version.toBase64(),
          });
        }
      } else if (input.type === DocumentSyncType.AWARENESS) {
        pubsub.publish('document:sync', input.documentId, {
          target: `!${input.clientId}`,
          type: DocumentSyncType.AWARENESS,
          data: input.data,
        });
      }

      return true;
    },
  }),

  checkSpellingDocument: t.withAuth({ session: true }).fieldWithInput({
    type: [
      builder.simpleObject('DocumentSpellingError', {
        fields: (t) => ({
          id: t.string(),
          nodeId: t.string(),
          startOffset: t.int(),
          endOffset: t.int(),
          context: t.string(),
          corrections: t.stringList(),
          explanation: t.string(),
        }),
      }),
    ],
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      text: t.input.string(),
      mappings: t.input.field({
        type: [
          builder.inputType('SpellcheckTextMappingInput', {
            fields: (t) => ({
              nodeId: t.string(),
              textStart: t.int(),
              textEnd: t.int(),
              blockOffset: t.int(),
            }),
          }),
        ],
      }),
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

      const { text, mappings } = input;
      if (!text.trim()) {
        return [];
      }

      const errors = await spellcheck.check(text);

      const findMapping = (position: number) => {
        let left = 0;
        let right = mappings.length - 1;

        while (left <= right) {
          const mid = (left + right) >> 1;
          const m = mappings[mid];

          if (position >= m.textStart && position < m.textEnd) {
            return m;
          }

          if (position < m.textStart) {
            right = mid - 1;
          } else {
            left = mid + 1;
          }
        }

        return;
      };

      const mapRange = (textStart: number, textEnd: number) => {
        const startMapping = findMapping(textStart);
        const endMapping = findMapping(textEnd - 1);

        if (!startMapping || !endMapping || startMapping.nodeId !== endMapping.nodeId) {
          return null;
        }

        const startOffset = startMapping.blockOffset + (textStart - startMapping.textStart);
        const endOffset = startMapping.blockOffset + (textEnd - startMapping.textStart);

        return { nodeId: startMapping.nodeId, startOffset, endOffset };
      };

      let errorId = 0;
      return errors
        .map((error) => {
          const range = mapRange(error.start, error.end);
          if (!range) return null;

          return {
            id: `err-${errorId++}`,
            nodeId: range.nodeId,
            startOffset: range.startOffset,
            endOffset: range.endOffset,
            context: error.context,
            corrections: error.corrections,
            explanation: error.explanation,
          };
        })
        .filter((error): error is NonNullable<typeof error> => error !== null);
    },
  }),
}));

builder.subscriptionFields((t) => ({
  documentSyncStream: t.withAuth({ session: true }).field({
    type: t.builder.simpleObject('DocumentSyncStreamPayload', {
      fields: (t) => ({
        documentId: t.id(),
        type: t.field({ type: DocumentSyncType }),
        data: t.string(),
      }),
    }),
    args: {
      clientId: t.arg.string(),
      documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
    },
    subscribe: async (_, args, ctx) => {
      const document = await db
        .select({ siteId: Entities.siteId, availability: Entities.availability })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, args.documentId))
        .then(firstOrThrow);

      if (document.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: document.siteId,
        });
      }

      pubsub.publish('document:sync', args.documentId, {
        target: `!${args.clientId}`,
        type: DocumentSyncType.PRESENCE,
        data: '',
      });

      const repeater = Repeater.merge([
        pubsub.subscribe('document:sync', args.documentId),
        new Repeater<{ target: string; type: DocumentSyncType; data: string }>(async (push, stop) => {
          const heartbeat = async () => {
            await redis.zadd('writers:active', Date.now(), ctx.session.userId);
            push({
              target: args.clientId,
              type: DocumentSyncType.HEARTBEAT,
              data: dayjs().toISOString(),
            });
          };

          await heartbeat();
          const interval = setInterval(heartbeat, 1000);

          await stop;

          clearInterval(interval);
        }),
      ]);

      return pipe(
        repeater,
        filter(({ target }) => {
          if (target === '*') {
            return true;
          } else if (target.startsWith('!')) {
            return target.slice(1) !== args.clientId;
          } else {
            return target === args.clientId;
          }
        }),
      );
    },
    resolve: async (payload, args) => {
      return {
        documentId: args.documentId,
        type: payload.type,
        data: payload.data,
      };
    },
  }),
}));

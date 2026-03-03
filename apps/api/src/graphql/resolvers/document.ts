import { createHash } from 'node:crypto';
import dayjs from 'dayjs';
import dedent from 'dedent';
import { and, asc, count, desc, eq, gt, gte, inArray, isNull, lt, sum } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { LoroDoc, VersionVector } from 'loro-crdt';
import { nanoid } from 'nanoid';
import { match } from 'ts-pattern';
import { redis } from '@/cache';
import {
  db,
  decodeDbId,
  DocumentArchivedNodes,
  DocumentCharacterCountChanges,
  DocumentContents,
  DocumentReactions,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Embeds,
  Entities,
  Files,
  first,
  firstOrThrow,
  firstOrThrowWith,
  Images,
  Notes,
  TableCode,
  UserPersonalIdentities,
  UserPreferences,
  Users,
  validateDbId,
} from '@/db';
import {
  DocumentAvailableAction,
  DocumentContentRating,
  DocumentSyncType,
  DocumentType,
  DocumentViewBodyUnavailableReason,
  EntityAvailability,
  EntityState,
  EntityType,
  EntityVisibility,
  NoteState,
} from '@/enums';
import { env } from '@/env';
import { NotFoundError, TypieError } from '@/errors';
import * as slack from '@/external/slack';
import * as spellcheck from '@/external/spellcheck';
import { enqueueJob } from '@/mq';
import { pubsub } from '@/pubsub';
import {
  extractAssetIdsFromLoroDoc,
  extractLoroDocContents,
  extractLoroDocLayoutMode,
  generateFractionalOrder,
  generatePermalink,
  generateSlug,
  getKoreanAge,
  makeLoroDoc,
} from '@/utils';
import { compressZstd, decompressZstd } from '@/utils/compression';
import { getDocumentFontFamilies } from '@/utils/document';
import { assertSitePermission } from '@/utils/permission';
import { assertPlanRule } from '@/utils/plan';
import { wasm } from '@/utils/wasm';
import { builder } from '../builder';
import {
  CharacterCountChange,
  Document,
  DocumentArchivedNode,
  DocumentFontFamily,
  DocumentReaction,
  DocumentVersion,
  DocumentView,
  Embed,
  Entity,
  EntityView,
  File,
  IDocument,
  Image,
  isTypeOf,
} from '../objects';
import type { Context } from '@/context';

const DocumentAsset = builder.loadableUnion('DocumentAsset', {
  types: [Image, File, Embed, DocumentArchivedNode],
  load: async (ids: string[]) => {
    const imageIds = ids.filter((id) => decodeDbId(id) === TableCode.IMAGES);
    const fileIds = ids.filter((id) => decodeDbId(id) === TableCode.FILES);
    const embedIds = ids.filter((id) => decodeDbId(id) === TableCode.EMBEDS);
    const archivedIds = ids.filter((id) => decodeDbId(id) === TableCode.DOCUMENT_ARCHIVED_NODES);

    const [images, files, embeds, archivedNodes] = await Promise.all([
      imageIds.length > 0 ? db.select().from(Images).where(inArray(Images.id, imageIds)) : [],
      fileIds.length > 0 ? db.select().from(Files).where(inArray(Files.id, fileIds)) : [],
      embedIds.length > 0 ? db.select().from(Embeds).where(inArray(Embeds.id, embedIds)) : [],
      archivedIds.length > 0 ? db.select().from(DocumentArchivedNodes).where(inArray(DocumentArchivedNodes.id, archivedIds)) : [],
    ]);

    return [...images, ...files, ...embeds, ...archivedNodes];
  },
  toKey: (item) => item.id,
  sort: true,
});

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

    assets: t.field({
      type: [DocumentAsset],
      resolve: async (self) => {
        const content = await db
          .select({ snapshot: DocumentContents.snapshot })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(firstOrThrow);

        const doc = new LoroDoc();
        doc.import(content.snapshot);
        const { imageIds, fileIds, embedIds } = extractAssetIdsFromLoroDoc(doc);

        const [existingImageIds, existingFileIds, existingEmbedIds] = await Promise.all([
          imageIds.length > 0
            ? db
                .select({ id: Images.id })
                .from(Images)
                .where(inArray(Images.id, imageIds))
                .then((r) => r.map((x) => x.id))
            : [],
          fileIds.length > 0
            ? db
                .select({ id: Files.id })
                .from(Files)
                .where(inArray(Files.id, fileIds))
                .then((r) => r.map((x) => x.id))
            : [],
          embedIds.length > 0
            ? db
                .select({ id: Embeds.id })
                .from(Embeds)
                .where(inArray(Embeds.id, embedIds))
                .then((r) => r.map((x) => x.id))
            : [],
        ]);

        return [...existingImageIds, ...existingFileIds, ...existingEmbedIds];
      },
    }),

    fontFamilies: t.field({
      type: [DocumentFontFamily],
      resolve: async (self) => {
        const entity = await db
          .select({ userId: Entities.userId })
          .from(Entities)
          .innerJoin(Documents, eq(Documents.entityId, Entities.id))
          .where(eq(Documents.id, self.id))
          .then(firstOrThrow);

        return await getDocumentFontFamilies(entity.userId);
      },
    }),
  }),
});

Document.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENTS),
  interfaces: [IDocument],
  fields: (t) => ({
    view: t.expose('id', { type: DocumentView }),
    password: t.exposeString('password', { nullable: true }),
    contentRating: t.expose('contentRating', { type: DocumentContentRating }),
    allowReaction: t.exposeBoolean('allowReaction'),
    protectContent: t.exposeBoolean('protectContent'),
    locked: t.exposeBoolean('locked'),

    assets: t.field({
      type: [DocumentAsset],
      resolve: async (self) => {
        const content = await db
          .select({ snapshot: DocumentContents.snapshot })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(firstOrThrow);

        const doc = new LoroDoc();
        doc.import(content.snapshot);
        const { imageIds, fileIds, embedIds, archivedIds } = extractAssetIdsFromLoroDoc(doc);

        const [existingImageIds, existingFileIds, existingEmbedIds, existingArchivedIds] = await Promise.all([
          imageIds.length > 0
            ? db
                .select({ id: Images.id })
                .from(Images)
                .where(inArray(Images.id, imageIds))
                .then((r) => r.map((x) => x.id))
            : [],
          fileIds.length > 0
            ? db
                .select({ id: Files.id })
                .from(Files)
                .where(inArray(Files.id, fileIds))
                .then((r) => r.map((x) => x.id))
            : [],
          embedIds.length > 0
            ? db
                .select({ id: Embeds.id })
                .from(Embeds)
                .where(inArray(Embeds.id, embedIds))
                .then((r) => r.map((x) => x.id))
            : [],
          archivedIds.length > 0
            ? db
                .select({ id: DocumentArchivedNodes.id })
                .from(DocumentArchivedNodes)
                .where(inArray(DocumentArchivedNodes.id, archivedIds))
                .then((r) => r.map((x) => x.id))
            : [],
        ]);

        return [...existingImageIds, ...existingFileIds, ...existingEmbedIds, ...existingArchivedIds];
      },
    }),

    thumbnail: t.field({
      type: Image,
      nullable: true,
      resolve: (self) => self.thumbnailId,
    }),

    reactionCount: t.int({
      resolve: async (self) => {
        const r = await db
          .select({ count: count() })
          .from(DocumentReactions)
          .where(eq(DocumentReactions.documentId, self.id))
          .then(firstOrThrow);
        return r.count;
      },
    }),

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

    version: t.field({
      type: 'Binary',
      resolve: async (self) => {
        const content = await db
          .select({ version: DocumentContents.version })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(firstOrThrow);
        return content.version;
      },
    }),

    generation: t.int({
      resolve: async (self) => {
        const content = await db
          .select({ generation: DocumentContents.generation })
          .from(DocumentContents)
          .where(eq(DocumentContents.documentId, self.id))
          .then(firstOrThrow);
        return content.generation;
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

    characterCount: t.int({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Document.characterCount',
          load: async (ids) => {
            return await db
              .select({ documentId: DocumentContents.documentId, characterCount: DocumentContents.characterCount })
              .from(DocumentContents)
              .where(inArray(DocumentContents.documentId, ids));
          },
          key: ({ documentId }) => documentId,
        });

        const content = await loader.load(self.id);
        return content.characterCount;
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

async function checkDocumentViewAccess(
  document: Pick<typeof Documents.$inferSelect, 'id' | 'contentRating' | 'password'>,
  ctx: Context,
): Promise<{ accessible: true } | { accessible: false; reason: DocumentViewBodyUnavailableReason }> {
  if (document.contentRating !== DocumentContentRating.ALL) {
    if (!ctx.session) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION };
    }

    const identity = await db
      .select({
        birthday: UserPersonalIdentities.birthDate,
        expiresAt: UserPersonalIdentities.expiresAt,
      })
      .from(UserPersonalIdentities)
      .where(eq(UserPersonalIdentities.userId, ctx.session.userId))
      .then(first);

    if (!identity) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION };
    }

    if (identity.expiresAt.isBefore(dayjs())) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION };
    }

    const minAge = match(document.contentRating)
      .with(DocumentContentRating.R15, () => 15)
      .with(DocumentContentRating.R19, () => 19)
      .exhaustive();

    if (getKoreanAge(identity.birthday) < minAge) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_MINIMUM_AGE };
    }
  }

  if (document.password !== null) {
    const passwordUnlock = await redis.get(
      getDocumentViewUnlockKey({
        documentId: document.id,
        deviceId: ctx.deviceId,
        password: document.password,
      }),
    );

    if (passwordUnlock !== 'true') {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_PASSWORD };
    }
  }

  return { accessible: true };
}

function getDocumentViewUnlockKey({ documentId, deviceId, password }: { documentId: string; deviceId: string; password: string }): string {
  const passwordHash = createHash('sha256').update(password).digest('hex');

  return `documentview:unlock:${documentId}:${deviceId}:${passwordHash}`;
}

DocumentView.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENTS),
  interfaces: [IDocument],
  fields: (t) => ({
    entity: t.expose('entityId', { type: EntityView }),
    hasPassword: t.boolean({ resolve: (self) => !!self.password }),
    protectContent: t.exposeBoolean('protectContent'),
    allowReaction: t.exposeBoolean('allowReaction'),

    thumbnail: t.field({
      type: Image,
      nullable: true,
      resolve: (self) => self.thumbnailId,
    }),

    availableActions: t.field({
      type: [DocumentAvailableAction],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'DocumentView.availableActions',
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: Documents.id, entityId: Entities.id, siteId: Entities.siteId })
              .from(Documents)
              .innerJoin(Entities, eq(Documents.entityId, Entities.id))
              .where(inArray(Documents.id, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        const document = await loader.load(self.id);

        return await Promise.allSettled([
          assertSitePermission({
            userId: ctx.session?.userId,
            siteId: document.siteId,
          }).then(() => DocumentAvailableAction.EDIT),
        ]).then((results) => results.filter((result) => result.status === 'fulfilled').flatMap((result) => result.value));
      },
    }),

    excerpt: t.string({
      resolve: async (self, _, ctx) => {
        const access = await checkDocumentViewAccess(self, ctx);
        if (!access.accessible) {
          return '(미리보기가 제한된 문서입니다)';
        }

        const loader = ctx.loader({
          name: 'DocumentView.excerpt',
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentContents.documentId, text: DocumentContents.text })
              .from(DocumentContents)
              .where(inArray(DocumentContents.documentId, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        const content = await loader.load(self.id);
        const text = content.text.replaceAll(/\s+/g, ' ').trim();

        return text.length <= 200 ? text : text.slice(0, 200) + '...';
      },
    }),

    snapshot: t.field({
      type: 'Binary',
      nullable: true,
      resolve: async (self, _, ctx) => {
        const access = await checkDocumentViewAccess(self, ctx);
        if (!access.accessible) {
          return null;
        }

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

    body: t.field({
      type: t.builder.unionType('DocumentViewBody', {
        types: [
          t.builder.simpleObject('DocumentViewBodyAvailable', {
            fields: (t) => ({ snapshot: t.field({ type: 'Binary' }) }),
          }),
          t.builder.simpleObject('DocumentViewBodyUnavailable', {
            fields: (t) => ({ reason: t.field({ type: DocumentViewBodyUnavailableReason }) }),
          }),
        ],
      }),
      resolve: async (self, _, ctx) => {
        const access = await checkDocumentViewAccess(self, ctx);
        if (!access.accessible) {
          return {
            __typename: 'DocumentViewBodyUnavailable' as const,
            reason: access.reason,
          };
        }

        const loader = ctx.loader({
          name: 'DocumentView.body',
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
          return {
            __typename: 'DocumentViewBodyUnavailable' as const,
            reason: DocumentViewBodyUnavailableReason.REQUIRE_PASSWORD,
          };
        }

        const doc = new LoroDoc();
        doc.import(content.snapshot);
        const snapshot = new Uint8Array(doc.export({ mode: 'shallow-snapshot', frontiers: doc.oplogFrontiers() }));

        return {
          __typename: 'DocumentViewBodyAvailable' as const,
          snapshot,
        };
      },
    }),

    reactions: t.field({
      type: [DocumentReaction],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'DocumentView.reactions',
          many: true,
          load: async (ids: string[]) => {
            return await db
              .select()
              .from(DocumentReactions)
              .where(inArray(DocumentReactions.documentId, ids))
              .orderBy(desc(DocumentReactions.createdAt));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        return await loader.load(self.id);
      },
    }),
  }),
});

DocumentArchivedNode.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENT_ARCHIVED_NODES),
  fields: (t) => ({
    id: t.exposeID('id'),
    content: t.exposeString('content'),
  }),
});

DocumentReaction.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENT_REACTIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    emoji: t.expose('emoji', { type: 'String' }),
    document: t.expose('documentId', { type: DocumentView }),
  }),
});

DocumentVersion.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENT_VERSIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    version: t.field({ type: 'Binary', resolve: async (self) => decompressZstd(self.version) }),
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

const SyncDocumentPayload = builder.simpleObject('SyncDocumentPayload', {
  fields: (t) => ({
    type: t.field({ type: DocumentSyncType }),
    data: t.string(),
  }),
});

builder.mutationFields((t) => ({
  createDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
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

      let orderLower: string | null = input.lowerOrder ?? null;

      if (!input.lowerOrder) {
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

        orderLower = last?.order ?? null;
      }

      const preference = await db
        .select({ value: UserPreferences.value })
        .from(UserPreferences)
        .where(eq(UserPreferences.userId, ctx.session.userId))
        .then(first);

      const template = (preference?.value as Record<string, unknown> | undefined)?.template as Record<string, unknown> | undefined;

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
            order: generateFractionalOrder({ lower: orderLower, upper: input.upperOrder ?? null }),
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

        const emptyDoc = makeLoroDoc(template);
        const snapshot = emptyDoc.export({ mode: 'snapshot' });
        const version = emptyDoc.version().encode();
        const { json, text, characterCount, blobSize } = await extractLoroDocContents(emptyDoc);

        await tx.insert(DocumentContents).values({
          documentId: document.id,
          json,
          text,
          characterCount,
          blobSize,
          snapshot,
          version,
        });

        const documentVersion = await tx
          .insert(DocumentVersions)
          .values({
            documentId: document.id,
            version: await compressZstd(version),
          })
          .returning({ id: DocumentVersions.id })
          .then(firstOrThrow);

        await tx.insert(DocumentVersionContributors).values({
          versionId: documentVersion.id,
          userId: ctx.session.userId,
        });

        return document;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });
      pubsub.publish('user:usage:update', ctx.session.userId, null);

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
      pubsub.publish('user:usage:update', ctx.session.userId, null);

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

        const json = await wasm.snapshotToJson(new Uint8Array(document.content.snapshot));
        const freshSnapshot = await wasm.jsonToSnapshot(json);
        const freshDoc = new LoroDoc();
        freshDoc.import(freshSnapshot);
        const freshVersion = freshDoc.version().encode();

        await tx.insert(DocumentContents).values({
          documentId: newDocument.id,
          json,
          text: document.content.text,
          characterCount: document.content.characterCount,
          blobSize: document.content.blobSize,
          snapshot: freshSnapshot,
          version: freshVersion,
        });

        // TODO: anchors

        const documentVersion = await tx
          .insert(DocumentVersions)
          .values({
            documentId: newDocument.id,
            version: await compressZstd(freshVersion),
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
      pubsub.publish('user:usage:update', ctx.session.userId, null);

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
      locked: t.input.boolean({ required: false }),
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
          ...(input.locked != null && { locked: input.locked }),
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
      password: t.input.string({ required: false }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
      contentRating: t.input.field({ type: DocumentContentRating, required: false }),
      allowReaction: t.input.boolean({ required: false }),
      protectContent: t.input.boolean({ required: false }),
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

        if (
          input.contentRating ||
          typeof input.allowReaction === 'boolean' ||
          typeof input.protectContent === 'boolean' ||
          input.password !== undefined ||
          input.thumbnailId !== undefined
        ) {
          await tx
            .update(Documents)
            .set({
              contentRating: input.contentRating ?? undefined,
              allowReaction: input.allowReaction ?? undefined,
              protectContent: input.protectContent ?? undefined,
              password: input.password,
              thumbnailId: input.thumbnailId,
            })
            .where(
              inArray(
                Documents.id,
                documents.map((doc) => doc.id),
              ),
            );
        }
      });

      pubsub.publish('site:update', siteId, { scope: 'site' });
      for (const doc of documents) {
        pubsub.publish('site:update', siteId, { scope: 'entity', entityId: doc.entityId });
      }

      return documents.map((doc) => doc.id);
    },
  }),

  syncDocument: t.withAuth({ session: true }).fieldWithInput({
    type: [SyncDocumentPayload],
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

      const { snapshot, version } = await db
        .select({
          snapshot: DocumentContents.snapshot,
          version: DocumentContents.version,
        })
        .from(DocumentContents)
        .where(eq(DocumentContents.documentId, input.documentId))
        .then(firstOrThrow);

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
        const clientVV = VersionVector.decode(Uint8Array.fromBase64(input.data));
        const doc = new LoroDoc();
        doc.import(snapshot);
        const updates = doc.export({ mode: 'update', from: clientVV });

        const updatesBase64 = updates.toBase64();
        const versionBase64 = version.toBase64();

        pubsub.publish('document:sync', input.documentId, {
          target: input.clientId,
          type: DocumentSyncType.UPDATE,
          data: updatesBase64,
        });

        pubsub.publish('document:sync', input.documentId, {
          target: input.clientId,
          type: DocumentSyncType.VECTOR,
          data: versionBase64,
        });

        return [
          { type: DocumentSyncType.UPDATE, data: updatesBase64 },
          { type: DocumentSyncType.VECTOR, data: versionBase64 },
        ];
      } else if (input.type === DocumentSyncType.AWARENESS) {
        pubsub.publish('document:sync', input.documentId, {
          target: `!${input.clientId}`,
          type: DocumentSyncType.AWARENESS,
          data: input.data,
        });
      }

      return [];
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

      return errors
        .map((error) => {
          const range = mapRange(error.start, error.end);
          if (!range) return null;

          return {
            id: nanoid(),
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

  unlockDocumentView: t.fieldWithInput({
    type: DocumentView,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      password: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({ password: Documents.password })
        .from(Documents)
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      if (document.password !== input.password) {
        throw new TypieError({ code: 'invalid_password' });
      }

      await redis.setex(
        getDocumentViewUnlockKey({
          documentId: input.documentId,
          deviceId: ctx.deviceId,
          password: document.password,
        }),
        60 * 60 * 24,
        'true',
      );

      return input.documentId;
    },
  }),

  createDocumentReaction: t.fieldWithInput({
    type: DocumentReaction,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      emoji: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({
          state: Entities.state,
          allowReaction: Documents.allowReaction,
        })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(first);

      if (document?.state !== EntityState.ACTIVE) {
        throw new TypieError({ code: 'not_found' });
      }

      if (!document.allowReaction) {
        throw new TypieError({ code: 'reaction_disallowed' });
      }

      return await db
        .insert(DocumentReactions)
        .values({
          documentId: input.documentId,
          userId: ctx.session?.userId,
          emoji: input.emoji,
          deviceId: ctx.deviceId,
        })
        .returning()
        .then(firstOrThrow);
    },
  }),

  reportDocument: t.fieldWithInput({
    type: 'Boolean',
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      reason: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({
          id: Documents.id,
          title: Documents.title,
          permalink: Entities.permalink,
        })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      const user = ctx.session
        ? await db
            .select({ id: Users.id, name: Users.name, email: Users.email })
            .from(Users)
            .where(eq(Users.id, ctx.session.userId))
            .then(firstOrThrow)
        : null;

      await slack.sendMessage({
        channel: '#cs',
        username: '타이피 신고 알림',
        iconEmoji: ':rotating_light:',
        message: dedent`
          *${document.title}* (${document.id}) 문서 신고
          *신고자:* ${user ? `${user.name} (${user.id}, ${user.email})` : `로그인하지 않은 사용자 (${ctx.ip})`}
          *이유:* ${input.reason ?? '(비어있음)'}
          ${env.USERSITE_URL.replace('*.', '')}/${document.permalink}
        `,
      });

      return true;
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

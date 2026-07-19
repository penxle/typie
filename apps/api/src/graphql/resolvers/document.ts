import { createHash, createHmac } from 'node:crypto';
import { setTimeout } from 'node:timers/promises';
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
  FontFamilySource,
  NoteState,
} from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import dedent from 'dedent';
import { and, asc, count, desc, eq, gt, gte, inArray, isNull, lt, sum } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { LoroDoc, VersionVector } from 'loro-crdt';
import { nanoid } from 'nanoid';
import qs from 'query-string';
import { match } from 'ts-pattern';
import { redis } from '#/cache.ts';
import {
  db,
  decodeDbId,
  DocumentArchivedNodes,
  DocumentBundles,
  DocumentCharacterCountChanges,
  DocumentContents,
  DocumentHeadContributors,
  DocumentHeads,
  DocumentReactions,
  Documents,
  DocumentStates,
  DocumentSweeps,
  DocumentVersionContributors,
  DocumentVersions,
  Embeds,
  Entities,
  Files,
  first,
  firstOrThrow,
  firstOrThrowWith,
  Images,
  NoteEntities,
  Notes,
  TableCode,
  UserPersonalIdentities,
  UserPreferences,
  Users,
  validateDbId,
} from '#/db/index.ts';
import { env } from '#/env.ts';
import * as slack from '#/external/slack.ts';
import * as spellcheck from '#/external/spellcheck.ts';
import { Lock } from '#/lock.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { appendBundle, getDurableHeads, readMergedGraph, setLiveHeads } from '#/utils/changeset.ts';
import { compressZstd, decompressZstd } from '#/utils/compression.ts';
import { getDocumentFontFamilies } from '#/utils/document.ts';
import { isSnapshotUsable } from '#/utils/document-state.ts';
import { isPrivateVisibilityOnlyInput } from '#/utils/documents-option-policy.ts';
import {
  buildFreshV2Content,
  calculateBlobSizeFromAssetIds,
  countCharacters,
  derivePlainRootFromPreset,
  extractAssetIdsFromPlainDoc,
  extractPlainDocLayoutMode,
  insertFreshV2Content,
} from '#/utils/entity.ts';
import {
  extractAssetIdsFromLoroDoc,
  extractLoroDocContents,
  extractLoroDocLayoutMode,
  generateFractionalOrder,
  generatePermalink,
  generateSlug,
  getKoreanAge,
  makeLoroDoc,
} from '#/utils/index.ts';
import { migrateDocumentToV2 } from '#/utils/migrate-v2.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { assertActiveSubscription, hasActiveSubscription } from '#/utils/plan.ts';
import { wasm } from '#/utils/wasm.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import { builder } from '../builder.ts';
import {
  CharacterCountChange,
  Document,
  DocumentArchivedNode,
  DocumentFontFamily,
  DocumentHead,
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
  User,
} from '../objects.ts';
import { resolveDocumentAssetsByIds } from './document-assets-by-ids.ts';
import type { PlainDoc } from '@typie/editor-ffi/server';
import type { Context } from '#/context.ts';
import type { TemplatePreset } from '#/utils/entity.ts';

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

type MaterializedDocumentAssetIds = {
  imageIds: string[];
  fileIds: string[];
  embedIds: string[];
  archivedIds: string[];
};

async function loadMaterializedDocumentAssetIds(documentId: string): Promise<MaterializedDocumentAssetIds> {
  const state = await db
    .select({ json: DocumentStates.json })
    .from(DocumentStates)
    .where(eq(DocumentStates.documentId, documentId))
    .then(first);

  let assetIds: { imageIds: string[]; fileIds: string[]; embedIds: string[]; archivedIds: string[] };
  if (state) {
    assetIds = extractAssetIdsFromPlainDoc(state.json as PlainDoc);
  } else {
    const content = await db
      .select({ snapshot: DocumentContents.snapshot })
      .from(DocumentContents)
      .where(eq(DocumentContents.documentId, documentId))
      .then(firstOrThrow);
    const doc = new LoroDoc();
    doc.import(content.snapshot);
    assetIds = extractAssetIdsFromLoroDoc(doc);
  }

  return assetIds;
}

async function loadExistingDocumentAssetIds({ imageIds, fileIds, embedIds, archivedIds }: MaterializedDocumentAssetIds): Promise<string[]> {
  const [existingImageIds, existingFileIds, existingEmbedIds, existingArchivedIds] = await Promise.all([
    imageIds.length > 0
      ? db
          .select({ id: Images.id })
          .from(Images)
          .where(inArray(Images.id, imageIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    fileIds.length > 0
      ? db
          .select({ id: Files.id })
          .from(Files)
          .where(inArray(Files.id, fileIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    embedIds.length > 0
      ? db
          .select({ id: Embeds.id })
          .from(Embeds)
          .where(inArray(Embeds.id, embedIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    archivedIds.length > 0
      ? db
          .select({ id: DocumentArchivedNodes.id })
          .from(DocumentArchivedNodes)
          .where(inArray(DocumentArchivedNodes.id, archivedIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
  ]);

  return [...existingImageIds, ...existingFileIds, ...existingEmbedIds, ...existingArchivedIds];
}

async function loadReferencedDocumentAssetIds(documentId: string): Promise<string[]> {
  return await loadExistingDocumentAssetIds(await loadMaterializedDocumentAssetIds(documentId));
}

async function loadOwnedDocumentAssetIds(userId: string, ids: string[]): Promise<string[]> {
  const imageIds = ids.filter((id) => decodeDbId(id) === TableCode.IMAGES);
  const fileIds = ids.filter((id) => decodeDbId(id) === TableCode.FILES);
  const embedIds = ids.filter((id) => decodeDbId(id) === TableCode.EMBEDS);

  const [ownedImageIds, ownedFileIds, ownedEmbedIds] = await Promise.all([
    imageIds.length > 0
      ? db
          .select({ id: Images.id })
          .from(Images)
          .where(and(eq(Images.userId, userId), inArray(Images.id, imageIds)))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    fileIds.length > 0
      ? db
          .select({ id: Files.id })
          .from(Files)
          .where(and(eq(Files.userId, userId), inArray(Files.id, fileIds)))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    embedIds.length > 0
      ? db
          .select({ id: Embeds.id })
          .from(Embeds)
          .where(and(eq(Embeds.userId, userId), inArray(Embeds.id, embedIds)))
          .then((rows) => rows.map(({ id }) => id))
      : [],
  ]);

  return [...ownedImageIds, ...ownedFileIds, ...ownedEmbedIds];
}

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
        const stateLoader = ctx.loader({
          name: 'Document.excerpt.v2',
          nullable: true,
          load: async (ids: string[]) => {
            return await db
              .select({
                documentId: DocumentStates.documentId,
                text: DocumentStates.text,
                projectionDegraded: DocumentStates.projectionDegraded,
              })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: (row) => row?.documentId,
        });

        const state = await stateLoader.load(self.id);

        let text: string;
        if (isSnapshotUsable(state)) {
          text = state.text.replaceAll(/\s+/g, ' ').trim();
        } else {
          const loader = ctx.loader({
            name: 'Document.excerpt',
            load: async (ids: string[]) => {
              return await db
                .select({ documentId: DocumentContents.documentId, text: DocumentContents.text })
                .from(DocumentContents)
                .where(inArray(DocumentContents.documentId, ids));
            },
            key: ({ documentId }: { documentId: string }) => documentId,
          });

          const content = await loader.load(self.id);
          text = content.text.replaceAll(/\s+/g, ' ').trim();
        }

        return text.length <= 200 ? text : text.slice(0, 200) + '...';
      },
    }),

    assets: t.field({
      type: [DocumentAsset],
      resolve: async (self) => {
        return await loadReferencedDocumentAssetIds(self.id);
      },
    }),

    assetsByIds: t.field({
      type: [DocumentAsset],
      args: { ids: t.arg.idList() },
      resolve: async (self, { ids }, ctx) => {
        return await resolveDocumentAssetsByIds({
          documentId: self.id,
          userId: ctx.session?.userId ?? null,
          requestedIds: ids,
          access: {
            loadOwnedIds: async ({ userId, ids }) => await loadOwnedDocumentAssetIds(userId, ids),
            loadReferencedIds: async ({ documentId }) => await loadReferencedDocumentAssetIds(documentId),
          },
        });
      },
    }),

    fontFamilies: t.field({
      type: [DocumentFontFamily],
      args: {
        sources: t.arg({
          type: [FontFamilySource],
          defaultValue: [FontFamilySource.DEFAULT, FontFamilySource.USER],
        }),
      },
      resolve: async (self, args, ctx) => {
        const entity = await db
          .select({ userId: Entities.userId })
          .from(Entities)
          .innerJoin(Documents, eq(Documents.entityId, Entities.id))
          .where(eq(Documents.id, self.id))
          .then(firstOrThrow);

        return await getDocumentFontFamilies(entity.userId, ctx.session?.userId ?? null, args.sources);
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

    thumbnail: t.field({
      type: Image,
      nullable: true,
      resolve: (self) => self.thumbnailId,
    }),

    previewUrl: t.string({
      resolve: (self) => {
        const now = Math.floor(Date.now() / 1000);
        const expires = Math.ceil(now / 3600) * 3600;
        const sig = createHmac('sha256', env.PREVIEW_SIGNING_SECRET).update(`${self.entityId}:${expires}`).digest('hex').slice(0, 16);
        return qs.stringifyUrl({
          url: `${env.API_URL}/entity/${self.entityId}/preview`,
          query: {
            expires,
            sig,
          },
        });
      },
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
        const state = await db
          .select({ json: DocumentStates.json, projectionDegraded: DocumentStates.projectionDegraded })
          .from(DocumentStates)
          .where(eq(DocumentStates.documentId, self.id))
          .then(first);

        if (isSnapshotUsable(state)) {
          return extractPlainDocLayoutMode(state.json as PlainDoc);
        }

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
        const stateLoader = ctx.loader({
          name: 'Document.characterCount.v2',
          nullable: true,
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentStates.documentId, characterCount: DocumentStates.characterCount })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: (row) => row?.documentId,
        });

        const state = await stateLoader.load(self.id);
        if (state) {
          return state.characterCount;
        }

        const loader = ctx.loader({
          name: 'Document.characterCount',
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentContents.documentId, characterCount: DocumentContents.characterCount })
              .from(DocumentContents)
              .where(inArray(DocumentContents.documentId, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
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

    heads: t.field({
      type: [DocumentHead],
      resolve: async (self) =>
        db.select().from(DocumentHeads).where(eq(DocumentHeads.documentId, self.id)).orderBy(desc(DocumentHeads.updatedAt)),
    }),

    sweepTombstones: t.stringList({
      resolve: async (self) => {
        const rows = await db
          .select({ zombieDots: DocumentSweeps.zombieDots })
          .from(DocumentSweeps)
          .where(eq(DocumentSweeps.documentId, self.id));
        return [...new Set(rows.flatMap((row) => row.zombieDots))];
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
    passwordUnlocked: t.boolean({
      resolve: async (self, _, ctx) => {
        if (!self.password) {
          return false;
        }

        const unlocked = await redis.get(
          getDocumentViewUnlockKey({
            documentId: self.id,
            deviceId: ctx.deviceId,
            password: self.password,
          }),
        );

        return unlocked === 'true';
      },
    }),
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
        ]).then((results) => results.flatMap((result) => (result.status === 'fulfilled' ? result.value : [])));
      },
    }),

    excerpt: t.string({
      resolve: async (self, _, ctx) => {
        const access = await checkDocumentViewAccess(self, ctx);
        if (!access.accessible) {
          return '(미리보기가 제한된 문서입니다)';
        }

        const stateLoader = ctx.loader({
          name: 'DocumentView.excerpt.v2',
          nullable: true,
          load: async (ids: string[]) => {
            return await db
              .select({
                documentId: DocumentStates.documentId,
                text: DocumentStates.text,
                projectionDegraded: DocumentStates.projectionDegraded,
              })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: (row) => row?.documentId,
        });

        const state = await stateLoader.load(self.id);

        let text: string;
        if (isSnapshotUsable(state)) {
          text = state.text.replaceAll(/\s+/g, ' ').trim();
        } else {
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
          text = content.text.replaceAll(/\s+/g, ' ').trim();
        }

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
          t.builder.simpleObject('DocumentViewBodyAvailableV2', {
            fields: (t) => ({ graph: t.field({ type: 'Binary' }) }),
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

        const state = await db
          .select({ documentId: DocumentStates.documentId })
          .from(DocumentStates)
          .where(eq(DocumentStates.documentId, self.id))
          .then(first);

        if (state) {
          const graph = await readMergedGraph(self.id);
          return {
            __typename: 'DocumentViewBodyAvailableV2' as const,
            graph,
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

    state: t.field({
      type: t.builder.simpleObject('DocumentViewState', {
        fields: (t) => ({
          updatedAt: t.field({ type: 'DateTime' }),
        }),
      }),
      nullable: true,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'DocumentView.state',
          nullable: true,
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentStates.documentId, updatedAt: DocumentStates.updatedAt })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: (row) => row?.documentId,
        });

        const row = await loader.load(self.id);
        if (!row) {
          return null;
        }

        return { updatedAt: row.updatedAt };
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

DocumentHead.implement({
  isTypeOf: isTypeOf(TableCode.DOCUMENT_HEADS),
  fields: (t) => ({
    id: t.exposeID('id'),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
    characterCount: t.exposeInt('characterCount'),
    heads: t.field({ type: 'Binary', resolve: (self) => self.heads }),
    contributors: t.field({
      type: [User],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'DocumentHead.contributors',
          many: true,
          load: async (ids: string[]) =>
            db
              .select({ headId: DocumentHeadContributors.headId, user: Users })
              .from(DocumentHeadContributors)
              .innerJoin(Users, eq(DocumentHeadContributors.userId, Users.id))
              .where(inArray(DocumentHeadContributors.headId, ids)),
          key: ({ headId }: { headId: string }) => headId,
        });

        const rows = await loader.load(self.id);
        return rows.map((row) => row.user);
      },
    }),
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
      v2: t.input.boolean({ required: false, defaultValue: false }),
    },
    resolve: async (_, { input }, ctx) => {
      if (!input.v2) {
        throw new TypieError({ code: 'v2_required' });
      }

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

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

      const preset = (preference?.value as Record<string, unknown> | undefined)?.template as TemplatePreset | undefined;

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
            icon: 'file',
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

        const emptyDoc = makeLoroDoc(preset);
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

        const { root, modifiers } = derivePlainRootFromPreset(preset);
        await wasmFfi.use(async (host) => {
          const plain = host.default_doc_with_preset(root, modifiers);
          const graph = host.to_graph(plain);
          const heads = host.heads(graph);
          const text = host.extract_text(plain);
          const { imageIds, fileIds } = extractAssetIdsFromPlainDoc(plain);
          const blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);
          const characterCount = countCharacters(text);

          await tx.insert(DocumentBundles).values({ documentId: document.id, seq: 1, payload: graph });
          await tx.insert(DocumentStates).values({
            documentId: document.id,
            json: plain,
            text,
            characterCount,
            blobSize,
            heads,
            lastBundleSeq: 1,
          });
        });

        return document;
      });

      if (input.parentEntityId) {
        pubsub.publish('site:update', input.siteId, { scope: 'entity', entityId: input.parentEntityId });
      } else {
        pubsub.publish('site:update', input.siteId, { scope: 'site' });
      }

      pubsub.publish('user:usage:update', ctx.session.userId, null);

      await enqueueJob('search:index:document', document.id);

      return document;
    },
  }),

  deleteDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: { documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ id: Entities.id, siteId: Entities.siteId, parentId: Entities.parentId })
        .from(Entities)
        .innerJoin(Documents, eq(Entities.id, Documents.entityId))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      await db
        .update(Entities)
        .set({
          state: EntityState.DELETED,
          deletedAt: dayjs(),
        })
        .where(eq(Entities.id, entity.id));

      if (entity.parentId) {
        pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.parentId });
      } else {
        pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      }
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.id });
      pubsub.publish('user:usage:update', ctx.session.userId, null);

      await enqueueJob('search:index:document', input.documentId);

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
          icon: Entities.icon,
          iconColor: Entities.iconColor,
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

      await assertActiveSubscription({ userId: ctx.session.userId });

      // TODO: anchors

      const noteRows = await db
        .select({
          content: Notes.content,
          color: Notes.color,
          status: Notes.status,
        })
        .from(NoteEntities)
        .innerJoin(Notes, eq(NoteEntities.noteId, Notes.id))
        .where(and(eq(NoteEntities.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)));

      const v2Content = await buildFreshV2Content(input.documentId);

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
            icon: entity.icon,
            iconColor: entity.iconColor,
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

        if (v2Content) {
          await insertFreshV2Content(tx, newDocument.id, v2Content);
        }

        if (noteRows.length > 0) {
          let prevOrder: string | null = null;

          for (const row of noteRows) {
            const order = generateFractionalOrder({ lower: prevOrder, upper: null });

            const newNote = await tx
              .insert(Notes)
              .values({
                userId: ctx.session.userId,
                siteId: entity.siteId,
                content: row.content,
                color: row.color,
                status: row.status,
                order,
              })
              .returning({ id: Notes.id })
              .then(firstOrThrow);

            await tx.insert(NoteEntities).values({
              noteId: newNote.id,
              entityId: newEntity.id,
            });

            prevOrder = order;
          }
        }

        return newDocument;
      });

      if (entity.parentEntityId) {
        pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.parentEntityId });
      } else {
        pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      }
      pubsub.publish('user:usage:update', ctx.session.userId, null);

      await enqueueJob('search:index:document', newDocument.id);

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

      await assertActiveSubscription({ userId: ctx.session.userId });

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

      await enqueueJob('search:index:document', input.documentId);

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
        .select({ siteId: Entities.siteId, entityId: Entities.id, parentId: Entities.parentId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: document.siteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

      const updatedDocument = await db
        .update(Documents)
        .set({ type: input.type })
        .where(eq(Documents.id, input.documentId))
        .returning()
        .then(firstOrThrow);

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
          parentId: Entities.parentId,
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

      if (!isPrivateVisibilityOnlyInput(input) && !(await hasActiveSubscription({ userId: ctx.session.userId }))) {
        throw new TypieError({ code: 'subscription_required', status: 403 });
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

      if (input.type === DocumentSyncType.UPDATE || input.type === DocumentSyncType.VECTOR) {
        const migrated = await db
          .select({ documentId: DocumentStates.documentId })
          .from(DocumentStates)
          .where(eq(DocumentStates.documentId, input.documentId))
          .then(first);

        if (migrated || (await redis.exists(`document:v2migrating:${input.documentId}`)) > 0) {
          throw new TypieError({ code: 'document_migrated' });
        }
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
        await assertActiveSubscription({ userId: ctx.session.userId });

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

  convertDocumentToV2: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select({ siteId: Entities.siteId, entityId: Entities.id })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: document.siteId });

      const lock = new Lock(`document:${input.documentId}`);
      const acquired = await lock.acquire();
      if (!acquired) {
        throw new TypieError({ code: 'document_busy' });
      }

      try {
        await redis.set(`document:v2migrating:${input.documentId}`, '1', 'EX', 900);
        await setTimeout(750);

        const pending = await redis.llen(`document:sync:updates:${input.documentId}`);
        if (pending > 0) {
          throw new TypieError({ code: 'document_busy' });
        }

        const result = await migrateDocumentToV2(input.documentId);
        if (result.status === 'skipped') {
          throw new TypieError({ code: 'already_migrated' });
        }
        if (result.status === 'failed') {
          throw new TypieError({ code: 'migration_failed', message: result.error });
        }

        const late = await redis.llen(`document:sync:updates:${input.documentId}`);
        if (late > 0) {
          console.warn(`convertDocumentToV2: ${late} legacy updates slipped in during migration of ${input.documentId}`);
        }
      } finally {
        await redis.del(`document:v2migrating:${input.documentId}`);
        await lock.release();
      }

      pubsub.publish('site:update', document.siteId, { scope: 'entity', entityId: document.entityId });

      await enqueueJob('search:index:document', input.documentId);
      await enqueueJob('document:preview:invalidate', input.documentId);

      return await db.select().from(Documents).where(eq(Documents.id, input.documentId)).then(firstOrThrow);
    },
  }),

  revertDocument: t.withAuth({ session: true }).fieldWithInput({
    type: builder.simpleObject('RevertDocumentPayload', {
      fields: (t) => ({
        heads: t.field({ type: 'Binary' }),
      }),
    }),
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      headId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_HEADS) }),
    },
    resolve: async (_, { input }, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      await assertActiveSubscription({ userId: ctx.session.userId });

      const head = await db
        .select({ heads: DocumentHeads.heads })
        .from(DocumentHeads)
        .where(and(eq(DocumentHeads.id, input.headId), eq(DocumentHeads.documentId, input.documentId)))
        .then(firstOrThrow);

      const graph = await readMergedGraph(input.documentId);

      const sweepRows = await db
        .select({ zombieDots: DocumentSweeps.zombieDots })
        .from(DocumentSweeps)
        .where(eq(DocumentSweeps.documentId, input.documentId));
      const sweepTombstones = [...new Set(sweepRows.flatMap((row) => row.zombieDots))];

      const { revert, opsCount, currentHeads } = await wasmFfi.use((host) => {
        const revert = host.revert(graph, head.heads, sweepTombstones);
        return { revert, opsCount: host.peek_changeset_ops_count(revert), currentHeads: host.heads(graph) };
      });

      if (opsCount === 0) {
        return { heads: currentHeads };
      }

      const seq = await appendBundle(input.documentId, revert, ctx.session.userId, ctx.session.deviceId);

      const mergedGraph = await readMergedGraph(input.documentId);
      const heads = await wasmFfi.use((host) => host.heads(mergedGraph));
      // No wasm recompute: the durable frontier is whatever collect has folded
      // into `document_states.heads` so far — the revert bundle itself only
      // affects it once collect processes this push, same as any other push.
      const durableHeads = (await getDurableHeads(input.documentId)) ?? new Uint8Array();

      await setLiveHeads(input.documentId, heads);

      pubsub.publish('document:changesets', input.documentId, {
        target: '*',
        seq,
        changesets: [revert.toBase64()],
        heads: heads.toBase64(),
        durableHeads: durableHeads.toBase64(),
      });

      await enqueueJob('document:changesets:collect', input.documentId);

      return { heads };
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

      await assertActiveSubscription({ userId: ctx.session.userId });

      const { text, mappings } = input;
      if (!text.trim()) {
        return [];
      }

      const errors = await spellcheck.check(text, ctx.c.req.raw.signal);

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

  checkSpellingDocumentV2: t.withAuth({ session: true }).fieldWithInput({
    type: [
      builder.simpleObject('DocumentSpellingErrorV2', {
        fields: (t) => ({
          id: t.string(),
          start: t.int(),
          end: t.int(),
          context: t.string(),
          corrections: t.stringList(),
          explanation: t.string(),
        }),
      }),
    ],
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      text: t.input.string(),
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

      await assertActiveSubscription({ userId: ctx.session.userId });

      const { text } = input;
      if (!text.trim()) {
        return [];
      }

      const errors = await spellcheck.check(text, ctx.c.req.raw.signal);

      const utf16ToCodepoint = (utf16Index: number): number => {
        let i = 0;
        let count = 0;
        while (i < utf16Index) {
          const cp = text.codePointAt(i);
          if (cp === undefined) break;
          i += cp > 0xff_ff ? 2 : 1;
          count++;
        }
        return count;
      };

      return errors.map((error) => ({
        id: nanoid(),
        start: utf16ToCodepoint(error.start),
        end: utf16ToCodepoint(error.end),
        context: error.context,
        corrections: error.corrections,
        explanation: error.explanation,
      }));
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
          }
          if (target.startsWith('!')) {
            return target.slice(1) !== args.clientId;
          }
          return target === args.clientId;
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

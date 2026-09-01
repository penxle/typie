import { createHash } from 'node:crypto';
import {
  DocumentAvailableAction,
  DocumentContentRating,
  DocumentType,
  DocumentViewBodyUnavailableReason,
  EntityAvailability,
  EntityState,
  EntityVisibility,
  FontFamilySource,
} from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import dedent from 'dedent';
import { and, count, desc, eq, gte, inArray, lt, sql, sum } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { match } from 'ts-pattern';
import { redis } from '#/cache.ts';
import {
  db,
  decodeDbId,
  DocumentArchivedNodes,
  DocumentCharacterCountChanges,
  DocumentHeadContributors,
  DocumentHeads,
  DocumentReactions,
  Documents,
  DocumentStates,
  DocumentSweeps,
  Embeds,
  Entities,
  Files,
  first,
  firstOrThrow,
  firstOrThrowWith,
  Images,
  TableCode,
  UserPersonalIdentities,
  Users,
  validateDbId,
} from '#/db/index.ts';
import { env } from '#/env.ts';
import * as slack from '#/external/slack.ts';
import * as spellcheck from '#/external/spellcheck.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { readMergedGraph } from '#/utils/changeset.ts';
import { getDocumentFontFamilies } from '#/utils/document.ts';
import { publishBundle } from '#/utils/document-bundle.ts';
import { extractAssetIdsFromPlainDoc, extractPlainDocLayoutMode } from '#/utils/entity.ts';
import { createDocumentCore, duplicateDocumentCore, updateDocumentCore, updateDocumentsOptionCore } from '#/utils/entity-actions.ts';
import { getExcludedDeltasByDate } from '#/utils/excluded-stats.ts';
import { getKoreanAge } from '#/utils/index.ts';
import { assertDocumentPermission, assertSitePermission } from '#/utils/permission.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { runPostCommitEffects } from '#/utils/post-commit.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import { builder } from '../builder.ts';
import {
  CharacterCountChange,
  Document,
  DocumentArchivedNode,
  DocumentFontFamily,
  DocumentHead,
  DocumentReaction,
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
import type { Context, SessionContext } from '#/context.ts';
import type { PostCommitEffect } from '#/utils/post-commit.ts';

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
    .then(firstOrThrow);

  return extractAssetIdsFromPlainDoc(state.json as PlainDoc);
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
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentStates.documentId, text: DocumentStates.text })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        const state = await stateLoader.load(self.id);
        const text = state.text.replaceAll(/\s+/g, ' ').trim();

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

    layoutMode: t.field({
      type: 'JSON',
      resolve: async (self) => {
        const state = await db
          .select({ json: DocumentStates.json })
          .from(DocumentStates)
          .where(eq(DocumentStates.documentId, self.id))
          .then(firstOrThrow);

        return extractPlainDocLayoutMode(state.json as PlainDoc);
      },
    }),

    characterCount: t.int({
      resolve: async (self, _, ctx) => {
        const stateLoader = ctx.loader({
          name: 'Document.characterCount.v2',
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentStates.documentId, characterCount: DocumentStates.characterCount })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        const state = await stateLoader.load(self.id);
        return state.characterCount;
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

        const excludedByDate = await getExcludedDeltasByDate({
          userId: ctx.session.userId,
          from: startOfDay,
          to: startOfDay.add(1, 'day'),
          documentId: document.id,
        });
        const excluded = excludedByDate.get(startOfDay.format('YYYY-MM-DD'));

        return {
          date: startOfDay,
          additions: (change.additions ?? 0) - (excluded?.additions ?? 0),
          deletions: (change.deletions ?? 0) - (excluded?.deletions ?? 0),
        };
      },
    }),

    entity: t.expose('entityId', { type: Entity }),

    heads: t.field({
      type: [DocumentHead],
      resolve: async (self) =>
        db
          .select()
          .from(DocumentHeads)
          .where(eq(DocumentHeads.documentId, self.id))
          .orderBy(desc(DocumentHeads.updatedAt), sql`${DocumentHeads.seq} DESC NULLS LAST`),
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
          load: async (ids: string[]) => {
            return await db
              .select({ documentId: DocumentStates.documentId, text: DocumentStates.text })
              .from(DocumentStates)
              .where(inArray(DocumentStates.documentId, ids));
          },
          key: ({ documentId }: { documentId: string }) => documentId,
        });

        const state = await stateLoader.load(self.id);
        const text = state.text.replaceAll(/\s+/g, ' ').trim();

        return text.length <= 200 ? text : text.slice(0, 200) + '...';
      },
    }),

    body: t.field({
      type: t.builder.unionType('DocumentViewBody', {
        types: [
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

        const graph = await readMergedGraph(self.id);
        if (graph.length === 0) {
          throw new NotFoundError();
        }

        return {
          __typename: 'DocumentViewBodyAvailableV2' as const,
          graph,
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

async function loadViewerContribution(ctx: Context & SessionContext, headId: string) {
  const loader = ctx.loader({
    name: 'DocumentHead.viewerContribution',
    nullable: true,
    load: async (ids: string[]) => {
      return await db
        .select({
          headId: DocumentHeadContributors.headId,
          excluded: DocumentHeadContributors.excluded,
          additions: DocumentHeadContributors.additions,
          deletions: DocumentHeadContributors.deletions,
        })
        .from(DocumentHeadContributors)
        .where(and(inArray(DocumentHeadContributors.headId, ids), eq(DocumentHeadContributors.userId, ctx.session.userId)));
    },
    key: (row) => row?.headId,
  });

  return await loader.load(headId);
}

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

    excluded: t.withAuth({ session: true }).field({
      type: 'Boolean',
      nullable: true,
      resolve: async (self, _, ctx) => {
        const row = await loadViewerContribution(ctx, self.id);
        return row && row.additions !== null ? row.excluded : null;
      },
    }),

    additions: t.withAuth({ session: true }).field({
      type: 'Int',
      nullable: true,
      resolve: async (self, _, ctx) => {
        const row = await loadViewerContribution(ctx, self.id);
        return row?.additions ?? null;
      },
    }),

    deletions: t.withAuth({ session: true }).field({
      type: 'Int',
      nullable: true,
      resolve: async (self, _, ctx) => {
        const row = await loadViewerContribution(ctx, self.id);
        return row?.deletions ?? null;
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

  documentById: t.withAuth({ session: true }).field({
    type: Document,
    args: { documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, args, ctx) => {
      await assertDocumentPermission({ userId: ctx.session.userId, documentId: args.documentId }).catch(() => {
        throw new NotFoundError();
      });

      return await db.select().from(Documents).where(eq(Documents.id, args.documentId)).then(firstOrThrow);
    },
  }),
}));

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

      return await createDocumentCore(db, {
        userId: ctx.session.userId,
        siteId: input.siteId,
        parentEntityId: input.parentEntityId ?? null,
        lowerOrder: input.lowerOrder ?? null,
        upperOrder: input.upperOrder ?? null,
      });
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
    resolve: async (_, { input }, ctx) => await duplicateDocumentCore(db, { userId: ctx.session.userId, documentId: input.documentId }),
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
      const effects: PostCommitEffect[] = [];

      const updatedDocument = await db.transaction(async (tx) => {
        const document = await updateDocumentCore(
          tx,
          { userId: ctx.session.userId, documentId: input.documentId, title: input.title, subtitle: input.subtitle },
          (effect) => {
            effects.push(effect);
          },
        );

        if (input.locked == null) {
          return document;
        }

        return await tx
          .update(Documents)
          .set({ locked: input.locked })
          .where(eq(Documents.id, input.documentId))
          .returning()
          .then(firstOrThrow);
      });

      const errors = await runPostCommitEffects(effects);
      if (errors.length > 0) {
        throw errors[0];
      }

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
    resolve: async (_, { input }, ctx) =>
      await updateDocumentsOptionCore(db, {
        userId: ctx.session.userId,
        documentIds: input.documentIds,
        availability: input.availability,
        visibility: input.visibility,
        password: input.password,
        thumbnailId: input.thumbnailId,
        contentRating: input.contentRating,
        allowReaction: input.allowReaction,
        protectContent: input.protectContent,
      }),
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

      const { heads } = await publishBundle(input.documentId, revert, ctx.session.userId, ctx.session.deviceId);

      return { heads };
    },
  }),

  updateDocumentHeadExclusion: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentHead,
    input: {
      headId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_HEADS) }),
      excluded: t.input.boolean(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertActiveSubscription({ userId: ctx.session.userId });

      const row = await db
        .select({ id: DocumentHeadContributors.id, additions: DocumentHeadContributors.additions })
        .from(DocumentHeadContributors)
        .where(and(eq(DocumentHeadContributors.headId, input.headId), eq(DocumentHeadContributors.userId, ctx.session.userId)))
        .then(first);

      if (!row) {
        throw new TypieError({ code: 'not_contributed' });
      }

      if (row.additions === null) {
        throw new TypieError({ code: 'head_delta_unavailable' });
      }

      await db.update(DocumentHeadContributors).set({ excluded: input.excluded }).where(eq(DocumentHeadContributors.id, row.id));
      pubsub.publish('user:usage:update', ctx.session.userId, null);

      return input.headId;
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

      return errors.map((error) => ({
        id: nanoid(),
        start: error.start,
        end: error.end,
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

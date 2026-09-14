import { DocumentAvailableAction } from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import { desc, eq, inArray } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentReactions,
  Documents,
  Entities,
  firstOrThrowWith,
  PublicationTags,
  Spaces,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { env } from '#/env.ts';
import { groupAssetIds, loadExistingDocumentAssetIds } from '#/utils/document-assets.ts';
import { checkDocumentViewAccess, getDocumentViewUnlockKey, RESTRICTED_EXCERPT } from '#/utils/document-view-access.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { buildLatestVersionGraphsQuery } from '#/utils/publication-core.ts';
import { buildCollectionNeighborQuery, buildPublishedPublicationByIdQuery, deriveExcerpt } from '#/utils/publication-view-core.ts';
import { spaceUrl } from '#/utils/usersite-core.ts';
import { builder } from '../builder.ts';
import { CollectionView, DocumentReaction, IEditorDocument, Image, isTypeOf, PublicationView, SpaceView } from '../objects.ts';
import { DocumentAsset, DocumentViewBody } from './document.ts';
import { latestVersionLoader } from './publication.ts';
import type { Context } from '#/context.ts';

const latestVersionGraphLoader = (ctx: Context) =>
  ctx.loader({
    name: 'PublicationView.latestVersionGraph',
    load: async (ids: string[]) => await buildLatestVersionGraphsQuery(db, { publicationIds: ids }),
    key: ({ publicationId }: { publicationId: string }) => publicationId,
  });

const documentLoader = (ctx: Context) =>
  ctx.loader({
    name: 'PublicationView.document',
    load: async (ids: string[]) => await db.select().from(Documents).where(inArray(Documents.id, ids)),
    key: ({ id }: { id: string }) => id,
  });

const entityLoader = (ctx: Context) =>
  ctx.loader({
    name: 'PublicationView.entity',
    load: async (ids: string[]) =>
      await db
        .select({ documentId: Documents.id, siteId: Entities.siteId, slug: Entities.slug })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(inArray(Documents.id, ids)),
    key: ({ documentId }: { documentId: string }) => documentId,
  });

const spaceLoader = (ctx: Context) =>
  ctx.loader({
    name: 'PublicationView.space',
    load: async (ids: string[]) => await db.select().from(Spaces).where(inArray(Spaces.id, ids)),
    key: ({ id }: { id: string }) => id,
  });

const isOwner = async (ctx: Context, siteId: string) =>
  await assertSitePermission({ userId: ctx.session?.userId, siteId })
    .then(() => true)
    .catch(() => false);

PublicationView.implement({
  isTypeOf: isTypeOf(TableCode.PUBLICATIONS),
  interfaces: [IEditorDocument],
  fields: (t) => ({
    documentId: t.exposeID('documentId'),
    title: t.string({
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        return version.title || '(제목 없음)';
      },
    }),
    subtitle: t.string({
      nullable: true,
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        return version.subtitle;
      },
    }),
    excerpt: t.string({
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        const access = await checkDocumentViewAccess(document, ctx);
        if (!access.accessible) return RESTRICTED_EXCERPT;
        const version = await latestVersionLoader(ctx).load(self.id);
        return deriveExcerpt(version.text, version.excerpt);
      },
    }),
    thumbnail: t.field({
      type: Image,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        return version.thumbnailId;
      },
    }),
    publishedAt: t.field({ type: 'DateTime', resolve: (self) => self.publishedAt ?? self.updatedAt }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
    tags: t.stringList({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'PublicationView.tags',
          many: true,
          load: async (ids: string[]) =>
            await db.select().from(PublicationTags).where(inArray(PublicationTags.publicationId, ids)).orderBy(PublicationTags.order),
          key: ({ publicationId }: { publicationId: string }) => publicationId,
        });
        const tags = await loader.load(self.id);
        return tags.map((tag) => tag.name);
      },
    }),
    collection: t.field({ type: CollectionView, nullable: true, resolve: (self) => self.collectionId }),
    prevInCollection: t.field({
      type: PublicationView,
      nullable: true,
      resolve: async (self) =>
        self.collectionId && self.collectionOrder
          ? await buildCollectionNeighborQuery(db, {
              collectionId: self.collectionId,
              collectionOrder: self.collectionOrder,
              direction: 'prev',
            }).then((rows) => rows[0] ?? null)
          : null,
    }),
    nextInCollection: t.field({
      type: PublicationView,
      nullable: true,
      resolve: async (self) =>
        self.collectionId && self.collectionOrder
          ? await buildCollectionNeighborQuery(db, {
              collectionId: self.collectionId,
              collectionOrder: self.collectionOrder,
              direction: 'next',
            }).then((rows) => rows[0] ?? null)
          : null,
    }),
    hasPassword: t.boolean({
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        return !!document.password;
      },
    }),
    passwordUnlocked: t.boolean({
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        if (!document.password) return false;
        const unlocked = await redis.get(
          getDocumentViewUnlockKey({ documentId: document.id, deviceId: ctx.deviceId, password: document.password }),
        );
        return unlocked === 'true';
      },
    }),
    protectContent: t.boolean({
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        return document.protectContent;
      },
    }),
    allowReaction: t.boolean({
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        return document.allowReaction;
      },
    }),
    reactions: t.field({
      type: [DocumentReaction],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'PublicationView.reactions',
          many: true,
          load: async (ids: string[]) =>
            await db
              .select()
              .from(DocumentReactions)
              .where(inArray(DocumentReactions.documentId, ids))
              .orderBy(desc(DocumentReactions.createdAt)),
          key: ({ documentId }: { documentId: string }) => documentId,
        });
        return await loader.load(self.documentId);
      },
    }),
    body: t.field({
      type: DocumentViewBody,
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        const access = await checkDocumentViewAccess(document, ctx);
        if (!access.accessible) {
          return { __typename: 'DocumentViewBodyUnavailable' as const, reason: access.reason };
        }
        const version = await latestVersionGraphLoader(ctx).load(self.id);
        return { __typename: 'DocumentViewBodyAvailableV2' as const, graph: version.graph };
      },
    }),
    assets: t.field({
      type: [DocumentAsset],
      resolve: async (self, _, ctx) => {
        const document = await documentLoader(ctx).load(self.documentId);
        const access = await checkDocumentViewAccess(document, ctx);
        if (!access.accessible) return [];
        const version = await latestVersionLoader(ctx).load(self.id);
        return await loadExistingDocumentAssetIds(groupAssetIds(version.assetIds));
      },
    }),
    availableActions: t.field({
      type: [DocumentAvailableAction],
      resolve: async (self, _, ctx) => {
        const entity = await entityLoader(ctx).load(self.documentId);
        return (await isOwner(ctx, entity.siteId)) ? [DocumentAvailableAction.EDIT] : [];
      },
    }),
    editUrl: t.string({
      nullable: true,
      resolve: async (self, _, ctx) => {
        const entity = await entityLoader(ctx).load(self.documentId);
        return (await isOwner(ctx, entity.siteId)) ? `${env.WEBSITE_URL}/${entity.slug}` : null;
      },
    }),
    url: t.string({
      resolve: async (self, _, ctx) => {
        const space = await spaceLoader(ctx).load(self.spaceId);
        return `${spaceUrl(env.USERSITE_URL, space.slug)}/p/${self.id}`;
      },
    }),
    space: t.expose('spaceId', { type: SpaceView }),
  }),
});

builder.mutationFields((t) => ({
  unlockPublicationView: t.fieldWithInput({
    type: PublicationView,
    input: {
      publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }),
      password: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await buildPublishedPublicationByIdQuery(db, { publicationId: input.publicationId }).then(
        firstOrThrowWith(new NotFoundError()),
      );

      const document = await documentLoader(ctx).load(publication.documentId);

      if (document.password === null || document.password !== input.password) {
        throw new TypieError({ code: 'invalid_password' });
      }

      await redis.setex(
        getDocumentViewUnlockKey({ documentId: document.id, deviceId: ctx.deviceId, password: document.password }),
        60 * 60 * 24,
        'true',
      );

      return input.publicationId;
    },
  }),
}));

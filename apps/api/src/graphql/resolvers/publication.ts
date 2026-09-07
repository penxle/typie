import { PublicationState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { asc, eq, inArray } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, Documents, Entities, firstOrThrow, PublicationTags, Spaces, TableCode, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { pubsub } from '#/pubsub.ts';
import { liveKey } from '#/utils/changeset.ts';
import {
  cancelScheduledPublicationCore,
  computeHasUnpublishedChanges,
  publishDocumentCore,
  unpublishDocumentCore,
  updatePublicationCore,
} from '#/utils/publication.ts';
import { buildLatestVersionMetadataQuery } from '#/utils/publication-core.ts';
import { builder } from '../builder.ts';
import { Collection, Document, Image, isTypeOf, Publication, Space } from '../objects.ts';
import type { Context } from '#/context.ts';

export const latestVersionLoader = (ctx: Context) =>
  ctx.loader({
    name: 'Publication.latestVersion',
    load: async (ids: string[]) => await buildLatestVersionMetadataQuery(db, { publicationIds: ids }),
    key: ({ publicationId }: { publicationId: string }) => publicationId,
  });

const liveHeadsLoader = (ctx: Context) =>
  ctx.loader({
    name: 'Publication.liveHeads',
    nullable: true,
    load: async (ids: string[]) => {
      const values = await redis.mget(ids.map((id) => liveKey(id)));
      return ids.map((id, index) => {
        const value = values[index];
        return { id, heads: value ? Uint8Array.fromBase64(value) : null };
      });
    },
    key: (row) => row?.id,
  });

const publishSiteUpdate = async (documentId: string) => {
  const row = await db
    .select({ entityId: Entities.id, siteId: Entities.siteId })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(eq(Documents.id, documentId))
    .then(firstOrThrow);
  pubsub.publish('site:update', row.siteId, { scope: 'entity', entityId: row.entityId });
};

Publication.implement({
  isTypeOf: isTypeOf(TableCode.PUBLICATIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: PublicationState }),
    document: t.expose('documentId', { type: Document }),
    space: t.expose('spaceId', { type: Space }),
    collection: t.field({ type: Collection, nullable: true, resolve: (self) => self.collectionId }),
    collectionOrder: t.exposeString('collectionOrder', { nullable: true }),
    pinnedOrder: t.exposeString('pinnedOrder', { nullable: true }),
    publishedAt: t.expose('publishedAt', { type: 'DateTime', nullable: true }),
    scheduledAt: t.expose('scheduledAt', { type: 'DateTime', nullable: true }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
    unpublishedAt: t.expose('unpublishedAt', { type: 'DateTime', nullable: true }),
    title: t.string({
      nullable: true,
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        return version.title;
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
      nullable: true,
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        return version.excerpt;
      },
    }),
    characterCount: t.int({
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        return version.characterCount;
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
    tags: t.stringList({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Publication.tags',
          many: true,
          load: async (ids: string[]) =>
            await db.select().from(PublicationTags).where(inArray(PublicationTags.publicationId, ids)).orderBy(asc(PublicationTags.order)),
          key: ({ publicationId }: { publicationId: string }) => publicationId,
        });
        const tags = await loader.load(self.id);
        return tags.map((tag) => tag.name);
      },
    }),
    url: t.string({
      resolve: async (self, _, ctx) => {
        const space = await ctx
          .loader({
            name: 'Publication.space',
            load: async (ids: string[]) => await db.select().from(Spaces).where(inArray(Spaces.id, ids)),
            key: ({ id }: { id: string }) => id,
          })
          .load(self.spaceId);
        return `${env.USERSITE_URL.replace('*', () => space.slug)}/p/${self.id}`;
      },
    }),
    hasUnpublishedChanges: t.boolean({
      resolve: async (self, _, ctx) => {
        const version = await latestVersionLoader(ctx).load(self.id);
        const document = await ctx
          .loader({
            name: 'Publication.document',
            load: async (ids: string[]) =>
              await db
                .select({ id: Documents.id, title: Documents.title, subtitle: Documents.subtitle })
                .from(Documents)
                .where(inArray(Documents.id, ids)),
            key: ({ id }: { id: string }) => id,
          })
          .load(self.documentId);
        const live = await liveHeadsLoader(ctx).load(document.id);
        return await computeHasUnpublishedChanges(document, version, live?.heads ?? null);
      },
    }),
  }),
});

builder.mutationFields((t) => ({
  publishDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      spaceId: t.input.id({ validate: validateDbId(TableCode.SPACES) }),
      tags: t.input.stringList(),
      collectionId: t.input.id({ required: false, validate: validateDbId(TableCode.COLLECTIONS) }),
      excerpt: t.input.string({ required: false }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
      scheduledAt: t.input.field({ type: 'DateTime', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await publishDocumentCore(db, {
        userId: ctx.session.userId,
        documentId: input.documentId,
        spaceId: input.spaceId,
        tags: input.tags,
        collectionId: input.collectionId,
        excerpt: input.excerpt,
        thumbnailId: input.thumbnailId,
        scheduledAt: input.scheduledAt ?? null,
        now: dayjs(),
      });
      await publishSiteUpdate(publication.documentId);
      return publication;
    },
  }),

  updatePublication: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: {
      publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }),
      tags: t.input.stringList(),
      collectionId: t.input.id({ required: false, validate: validateDbId(TableCode.COLLECTIONS) }),
      excerpt: t.input.string({ required: false }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await updatePublicationCore(db, { userId: ctx.session.userId, ...input, now: dayjs() });
      await publishSiteUpdate(publication.documentId);
      return publication;
    },
  }),

  unpublishDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: { documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const publication = await unpublishDocumentCore(db, { userId: ctx.session.userId, documentId: input.documentId, now: dayjs() });
      await publishSiteUpdate(publication.documentId);
      return publication;
    },
  }),

  cancelScheduledPublication: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: { publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }) },
    resolve: async (_, { input }, ctx) => {
      const publication = await cancelScheduledPublicationCore(db, {
        userId: ctx.session.userId,
        publicationId: input.publicationId,
        now: dayjs(),
      });
      await publishSiteUpdate(publication.documentId);
      return publication;
    },
  }),
}));

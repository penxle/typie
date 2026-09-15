import { PublicationState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { asc, eq, inArray } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, Documents, Entities, PublicationTags, Sites, TableCode, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { pubsub } from '#/pubsub.ts';
import { liveKey } from '#/utils/changeset.ts';
import { enqueueDiscoveryPublicationSync } from '#/utils/discovery-index.ts';
import {
  cancelScheduledPublicationCore,
  computeHasUnpublishedChanges,
  findFolderDocumentIds,
  publishDocumentCore,
  publishDocumentsCore,
  unpublishDocumentCore,
  unpublishDocumentsCore,
  updatePublicationCore,
} from '#/utils/publication.ts';
import { buildLatestVersionMetadataQuery } from '#/utils/publication-core.ts';
import { pinPublicationCore, unpinPublicationCore } from '#/utils/publication-pin.ts';
import { publicationUrl } from '#/utils/usersite-core.ts';
import { builder } from '../builder.ts';
import { Document, Image, isTypeOf, Publication, Site } from '../objects.ts';
import type { Context } from '#/context.ts';

export const latestVersionLoader = (ctx: Context) =>
  ctx.loader({
    name: 'Publication.latestVersion',
    load: async (ids: string[]) => await buildLatestVersionMetadataQuery(db, { publicationIds: ids }),
    key: ({ publicationId }: { publicationId: string }) => publicationId,
  });

export const publicationEntityLoader = (ctx: Context) =>
  ctx.loader({
    name: 'Publication.entity',
    load: async (ids: string[]) =>
      await db
        .select({ documentId: Documents.id, entityId: Entities.id, siteId: Entities.siteId, slug: Entities.slug, number: Entities.number })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(inArray(Documents.id, ids)),
    key: ({ documentId }: { documentId: string }) => documentId,
  });

const siteSlugLoader = (ctx: Context) =>
  ctx.loader({
    name: 'Publication.siteSlug',
    load: async (ids: string[]) => await db.select({ id: Sites.id, slug: Sites.slug }).from(Sites).where(inArray(Sites.id, ids)),
    key: ({ id }: { id: string }) => id,
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

const publishSiteUpdate = async (documentIds: string[]) => {
  if (documentIds.length === 0) return;
  const rows = await db
    .select({ entityId: Entities.id, siteId: Entities.siteId })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(inArray(Documents.id, documentIds));
  for (const row of rows) {
    pubsub.publish('site:update', row.siteId, { scope: 'entity', entityId: row.entityId });
  }
};

const afterPublicationChange = async (publications: { id: string; documentId: string }[]) => {
  await enqueueDiscoveryPublicationSync(publications.map((publication) => publication.id));
  await publishSiteUpdate(publications.map((publication) => publication.documentId));
};

Publication.implement({
  isTypeOf: isTypeOf(TableCode.PUBLICATIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: PublicationState }),
    document: t.expose('documentId', { type: Document }),
    site: t.expose('siteId', { type: Site }),
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
        const [site, entity] = await Promise.all([
          siteSlugLoader(ctx).load(self.siteId),
          publicationEntityLoader(ctx).load(self.documentId),
        ]);
        return publicationUrl(env.USERSITE_URL, site.slug, entity.number);
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
      tags: t.input.stringList(),
      excerpt: t.input.string({ required: false }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
      scheduledAt: t.input.field({ type: 'DateTime', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await publishDocumentCore(db, {
        userId: ctx.session.userId,
        documentId: input.documentId,
        tags: input.tags,
        excerpt: input.excerpt,
        thumbnailId: input.thumbnailId,
        scheduledAt: input.scheduledAt ?? null,
        now: dayjs(),
      });
      await afterPublicationChange([publication]);
      return publication;
    },
  }),

  updatePublication: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: {
      publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }),
      tags: t.input.stringList(),
      excerpt: t.input.string({ required: false }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await updatePublicationCore(db, { userId: ctx.session.userId, ...input, now: dayjs() });
      await afterPublicationChange([publication]);
      return publication;
    },
  }),

  unpublishDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: { documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const publication = await unpublishDocumentCore(db, { userId: ctx.session.userId, documentId: input.documentId, now: dayjs() });
      await afterPublicationChange([publication]);
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
      await afterPublicationChange([publication]);
      return publication;
    },
  }),

  publishDocuments: t.withAuth({ session: true }).fieldWithInput({
    type: [Publication],
    input: {
      documentIds: t.input.idList({ validate: { items: validateDbId(TableCode.DOCUMENTS) } }),
      addTags: t.input.stringList({ required: false }),
      removeTags: t.input.stringList({ required: false }),
      scheduledAt: t.input.field({ type: 'DateTime', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const publications = await publishDocumentsCore(db, {
        userId: ctx.session.userId,
        documentIds: input.documentIds,
        addTags: input.addTags ?? undefined,
        removeTags: input.removeTags ?? undefined,
        scheduledAt: input.scheduledAt,
        now: dayjs(),
      });
      await afterPublicationChange(publications);
      return publications;
    },
  }),

  unpublishDocuments: t.withAuth({ session: true }).fieldWithInput({
    type: [Publication],
    input: { documentIds: t.input.idList({ validate: { items: validateDbId(TableCode.DOCUMENTS) } }) },
    resolve: async (_, { input }, ctx) => {
      const publications = await unpublishDocumentsCore(db, { userId: ctx.session.userId, documentIds: input.documentIds, now: dayjs() });
      await afterPublicationChange(publications);
      return publications;
    },
  }),

  publishFolderDocuments: t.withAuth({ session: true }).fieldWithInput({
    type: [Publication],
    input: {
      folderIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }),
      addTags: t.input.stringList({ required: false }),
      removeTags: t.input.stringList({ required: false }),
      scheduledAt: t.input.field({ type: 'DateTime', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const documentIds = await findFolderDocumentIds(db, { userId: ctx.session.userId, folderIds: input.folderIds });
      const publications = await publishDocumentsCore(db, {
        userId: ctx.session.userId,
        documentIds,
        addTags: input.addTags ?? undefined,
        removeTags: input.removeTags ?? undefined,
        scheduledAt: input.scheduledAt,
        now: dayjs(),
      });
      await afterPublicationChange(publications);
      return publications;
    },
  }),

  unpublishFolderDocuments: t.withAuth({ session: true }).fieldWithInput({
    type: [Publication],
    input: { folderIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }) },
    resolve: async (_, { input }, ctx) => {
      const documentIds = await findFolderDocumentIds(db, { userId: ctx.session.userId, folderIds: input.folderIds });
      const publications = await unpublishDocumentsCore(db, { userId: ctx.session.userId, documentIds, now: dayjs() });
      await afterPublicationChange(publications);
      return publications;
    },
  }),

  pinPublication: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: {
      publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await pinPublicationCore(db, { userId: ctx.session.userId, ...input });
      pubsub.publish('site:update', publication.siteId, { scope: 'site' });
      return publication;
    },
  }),

  unpinPublication: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: { publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }) },
    resolve: async (_, { input }, ctx) => {
      const publication = await unpinPublicationCore(db, { userId: ctx.session.userId, publicationId: input.publicationId });
      pubsub.publish('site:update', publication.siteId, { scope: 'site' });
      return publication;
    },
  }),
}));

builder.queryFields((t) => ({
  folderDocuments: t.withAuth({ session: true }).field({
    type: [Document],
    args: { folderIds: t.arg.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }) },
    resolve: async (_, args, ctx) => {
      const documentIds = await findFolderDocumentIds(db, { userId: ctx.session.userId, folderIds: args.folderIds });
      return documentIds;
    },
  }),
}));

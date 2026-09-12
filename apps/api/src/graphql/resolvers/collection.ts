import { asc, eq, inArray } from 'drizzle-orm';
import { db, Publications, Spaces, TableCode, validateDbId } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import {
  createCollectionCore,
  deleteCollectionCore,
  movePublicationInCollectionCore,
  pinPublicationCore,
  unpinPublicationCore,
  updateCollectionCore,
} from '#/utils/collection.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { findActiveSpace } from '#/utils/space.ts';
import { builder } from '../builder.ts';
import { Collection, Image, isTypeOf, Publication, Space } from '../objects.ts';

Collection.implement({
  isTypeOf: isTypeOf(TableCode.COLLECTIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    description: t.exposeString('description', { nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    cover: t.field({ type: Image, nullable: true, resolve: (self) => self.coverId }),
    space: t.expose('spaceId', { type: Space }),
    publications: t.field({
      type: [Publication],
      resolve: async (self, _, ctx) => {
        const space = await ctx
          .loader({
            name: 'Collection.space',
            load: async (ids: string[]) =>
              await db.select({ id: Spaces.id, siteId: Spaces.siteId }).from(Spaces).where(inArray(Spaces.id, ids)),
            key: ({ id }: { id: string }) => id,
          })
          .load(self.spaceId);

        await assertSitePermission({ userId: ctx.session?.userId, siteId: space.siteId });

        return await db
          .select()
          .from(Publications)
          .where(eq(Publications.collectionId, self.id))
          .orderBy(asc(Publications.collectionOrder));
      },
    }),
  }),
});

const siteOf = async (spaceId: string) => {
  const space = await findActiveSpace(db, spaceId);
  return space?.siteId;
};

builder.mutationFields((t) => ({
  createCollection: t.withAuth({ session: true }).fieldWithInput({
    type: Collection,
    input: {
      spaceId: t.input.id({ validate: validateDbId(TableCode.SPACES) }),
      name: t.input.string(),
      description: t.input.string({ required: false }),
      coverId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const collection = await createCollectionCore(db, { userId: ctx.session.userId, ...input });
      const siteId = await siteOf(collection.spaceId);
      if (siteId) pubsub.publish('site:update', siteId, { scope: 'site' });
      return collection;
    },
  }),

  updateCollection: t.withAuth({ session: true }).fieldWithInput({
    type: Collection,
    input: {
      collectionId: t.input.id({ validate: validateDbId(TableCode.COLLECTIONS) }),
      name: t.input.string({ required: false }),
      description: t.input.string({ required: false }),
      coverId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const collection = await updateCollectionCore(db, { userId: ctx.session.userId, ...input });
      const siteId = await siteOf(collection.spaceId);
      if (siteId) pubsub.publish('site:update', siteId, { scope: 'site' });
      return collection;
    },
  }),

  deleteCollection: t.withAuth({ session: true }).fieldWithInput({
    type: Collection,
    input: { collectionId: t.input.id({ validate: validateDbId(TableCode.COLLECTIONS) }) },
    resolve: async (_, { input }, ctx) => {
      const collection = await deleteCollectionCore(db, { userId: ctx.session.userId, collectionId: input.collectionId });
      const siteId = await siteOf(collection.spaceId);
      if (siteId) pubsub.publish('site:update', siteId, { scope: 'site' });
      return collection;
    },
  }),

  movePublicationInCollection: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: {
      publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const publication = await movePublicationInCollectionCore(db, { userId: ctx.session.userId, ...input });
      const siteId = await siteOf(publication.spaceId);
      if (siteId) pubsub.publish('site:update', siteId, { scope: 'site' });
      return publication;
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
      const siteId = await siteOf(publication.spaceId);
      if (siteId) pubsub.publish('site:update', siteId, { scope: 'site' });
      return publication;
    },
  }),

  unpinPublication: t.withAuth({ session: true }).fieldWithInput({
    type: Publication,
    input: { publicationId: t.input.id({ validate: validateDbId(TableCode.PUBLICATIONS) }) },
    resolve: async (_, { input }, ctx) => {
      const publication = await unpinPublicationCore(db, { userId: ctx.session.userId, publicationId: input.publicationId });
      const siteId = await siteOf(publication.spaceId);
      if (siteId) pubsub.publish('site:update', siteId, { scope: 'site' });
      return publication;
    },
  }),
}));

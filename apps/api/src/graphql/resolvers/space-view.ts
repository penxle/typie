import { SpaceAvailableAction } from '@typie/lib/enums';
import { NotFoundError } from '@typie/lib/errors';
import { and, eq, inArray } from 'drizzle-orm';
import { Collections, db, firstOrThrowWith, Sites, TableCode, validateDbId } from '#/db/index.ts';
import { buildPinnedPublicationsQuery } from '#/utils/collection-core.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import {
  buildCollectionsByIdsQuery,
  buildIndexableSpaceSlugsQuery,
  buildPublishedCollectionIdsQuery,
  buildPublishedPublicationCountQuery,
  buildPublishedPublicationsQuery,
  buildSitemapPaths,
  buildSpaceBySlugQuery,
  buildSpaceTagQuery,
  buildSpaceTagsQuery,
  clampPageSize,
  toPublicationsPage,
} from '#/utils/publication-view-core.ts';
import { builder } from '../builder.ts';
import { CollectionView, Image, ISpace, isTypeOf, PublicationView, SpaceView } from '../objects.ts';

export const SpacePublicationsPage = builder.simpleObject('SpacePublicationsPage', {
  fields: (t) => ({
    publications: t.field({ type: [PublicationView] }),
    hasMore: t.boolean(),
  }),
});

const loadPage = async (input: {
  spaceId: string;
  tagName?: string;
  excludePinned?: boolean;
  first?: number | null;
  after?: string | null;
}) => {
  const limit = clampPageSize(input.first);
  const rows = await buildPublishedPublicationsQuery(db, {
    spaceId: input.spaceId,
    tagName: input.tagName,
    excludePinned: input.excludePinned,
    after: input.after ?? null,
    limit: limit + 1,
  });
  return toPublicationsPage(rows, limit);
};

const SpaceTag = builder.objectRef<{ spaceId: string; name: string; count: number }>('SpaceTag').implement({
  fields: (t) => ({
    name: t.exposeString('name'),
    count: t.exposeInt('count'),
    publications: t.field({
      type: SpacePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (self, args) => await loadPage({ spaceId: self.spaceId, tagName: self.name, first: args.first, after: args.after }),
    }),
  }),
});

CollectionView.implement({
  isTypeOf: isTypeOf(TableCode.COLLECTIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    description: t.exposeString('description', { nullable: true }),
    cover: t.field({ type: Image, nullable: true, resolve: (self) => self.coverId }),
    publications: t.field({
      type: [PublicationView],
      resolve: async (self) =>
        await buildPublishedPublicationsQuery(db, { spaceId: self.spaceId, collectionId: self.id, after: null, limit: 1000 }),
    }),
  }),
});

SpaceView.implement({
  isTypeOf: isTypeOf(TableCode.SPACES),
  interfaces: [ISpace],
  fields: (t) => ({
    logo: t.field({
      type: Image,
      nullable: false,
      resolve: async (self, _, ctx) => {
        if (self.logoId) return self.logoId;

        const site = await ctx
          .loader({
            name: 'SpaceView.site',
            load: async (ids: string[]) =>
              await db.select({ id: Sites.id, logoId: Sites.logoId }).from(Sites).where(inArray(Sites.id, ids)),
            key: ({ id }: { id: string }) => id,
          })
          .load(self.siteId);

        return site.logoId;
      },
    }),
    availableActions: t.field({
      type: [SpaceAvailableAction],
      resolve: async (self, _, ctx) => {
        const owner = await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId })
          .then(() => true)
          .catch(() => false);
        return owner ? [SpaceAvailableAction.SETTINGS] : [];
      },
    }),
    pinnedPublications: t.field({
      type: [PublicationView],
      resolve: async (self) => await buildPinnedPublicationsQuery(db, { spaceIds: [self.id] }),
    }),
    publicationCount: t.int({
      resolve: async (self) => await buildPublishedPublicationCountQuery(db, { spaceId: self.id }).then((rows) => rows[0]?.count ?? 0),
    }),
    publications: t.field({
      type: SpacePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (self, args) => await loadPage({ spaceId: self.id, excludePinned: true, first: args.first, after: args.after }),
    }),
    publication: t.field({
      type: PublicationView,
      args: { publicationId: t.arg.id({ validate: validateDbId(TableCode.PUBLICATIONS) }) },
      resolve: async (self, args) =>
        await buildPublishedPublicationsQuery(db, { spaceId: self.id, publicationId: args.publicationId, after: null, limit: 1 }).then(
          firstOrThrowWith(new NotFoundError()),
        ),
    }),
    collections: t.field({
      type: [CollectionView],
      resolve: async (self) => {
        const ids = await buildPublishedCollectionIdsQuery(db, { spaceId: self.id }).then((rows) =>
          rows.flatMap((row) => (row.collectionId ? [row.collectionId] : [])),
        );
        if (ids.length === 0) return [];
        return await buildCollectionsByIdsQuery(db, { collectionIds: ids });
      },
    }),
    collection: t.field({
      type: CollectionView,
      args: { collectionId: t.arg.id({ validate: validateDbId(TableCode.COLLECTIONS) }) },
      resolve: async (self, args) =>
        await db
          .select()
          .from(Collections)
          .where(and(eq(Collections.id, args.collectionId), eq(Collections.spaceId, self.id)))
          .then(firstOrThrowWith(new NotFoundError())),
    }),
    tags: t.field({
      type: [SpaceTag],
      resolve: async (self) => {
        const rows = await buildSpaceTagsQuery(db, { spaceId: self.id });
        return rows.map((row) => ({ ...row, spaceId: self.id }));
      },
    }),
    tag: t.field({
      type: SpaceTag,
      args: { name: t.arg.string() },
      resolve: async (self, args) => {
        const name = args.name.normalize('NFC').trim();
        const found = await buildSpaceTagQuery(db, { spaceId: self.id, name }).then((rows) => rows[0]);
        if (!found) throw new NotFoundError();
        return { ...found, spaceId: self.id };
      },
    }),
    sitemap: t.stringList({
      resolve: async (self) => {
        if (!self.allowIndexing) return [];
        const [publications, collections, tags] = await Promise.all([
          buildPublishedPublicationsQuery(db, { spaceId: self.id, after: null, limit: 5000 }),
          buildPublishedCollectionIdsQuery(db, { spaceId: self.id }),
          buildSpaceTagsQuery(db, { spaceId: self.id }),
        ]);
        return buildSitemapPaths({
          publicationIds: publications.map((row) => row.id),
          collectionIds: collections.flatMap((row) => (row.collectionId ? [row.collectionId] : [])),
          tagNames: tags.map((row) => row.name),
        });
      },
    }),
  }),
});

builder.queryFields((t) => ({
  spaceView: t.field({
    type: SpaceView,
    args: { slug: t.arg.string() },
    resolve: async (_, args) => {
      return await buildSpaceBySlugQuery(db, { slug: args.slug }).then(firstOrThrowWith(new NotFoundError()));
    },
  }),

  sitemapSpaceSlugs: t.stringList({
    resolve: async () => {
      const rows = await buildIndexableSpaceSlugsQuery(db);
      return rows.map((row) => row.slug);
    },
  }),
}));

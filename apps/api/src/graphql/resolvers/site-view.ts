import { EntityType, SiteAvailableAction } from '@typie/lib/enums';
import { NotFoundError } from '@typie/lib/errors';
import { and, eq, inArray } from 'drizzle-orm';
import { db, Entities, firstOrThrowWith, Folders, Sites, TableCode, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import {
  buildIndexableSiteSlugsQuery,
  buildPinnedPublicationsQuery,
  buildPublishedPublicationCountQuery,
  buildPublishedPublicationNumbersQuery,
  buildPublishedPublicationsByEntityIdsQuery,
  buildPublishedPublicationsQuery,
  buildSiteBySlugQuery,
  buildSitemapPaths,
  buildSiteTagQuery,
  buildSiteTagsQuery,
  clampPageSize,
  toPublicationsPage,
} from '#/utils/publication-view-core.ts';
import {
  buildSiteTree,
  buildSiteTreeRowsQuery,
  visibleAncestors,
  visibleChildCounts,
  visibleChildren,
  visibleFolderIds,
} from '#/utils/site-tree-core.ts';
import { folderUrl } from '#/utils/usersite-core.ts';
import { builder } from '../builder.ts';
import { Image, ISite, isTypeOf, PublicationView, SiteFolderView, SiteView } from '../objects.ts';
import type { Context } from '#/context.ts';
import type { SiteTree, SiteTreeRow } from '#/utils/site-tree-core.ts';

export const SitePublicationsPage = builder.simpleObject('SitePublicationsPage', {
  fields: (t) => ({
    publications: t.field({ type: [PublicationView] }),
    hasMore: t.boolean(),
  }),
});

export const siteTreeLoader = (ctx: Context) =>
  ctx.loader({
    name: 'SiteView.tree',
    load: async (ids: string[]) => {
      const rows = await buildSiteTreeRowsQuery(db, { siteIds: ids });
      const bySite = new Map<string, SiteTreeRow[]>();
      for (const row of rows) bySite.set(row.siteId, [...(bySite.get(row.siteId) ?? []), row]);
      return ids.map((id) => ({ id, tree: buildSiteTree(bySite.get(id) ?? []) }));
    },
    key: ({ id }: { id: string }) => id,
  });

export const siteLoader = (ctx: Context) =>
  ctx.loader({
    name: 'SiteView.site',
    load: async (ids: string[]) => await db.select().from(Sites).where(inArray(Sites.id, ids)),
    key: ({ id }: { id: string }) => id,
  });

const folderLoader = (ctx: Context) =>
  ctx.loader({
    name: 'SiteFolderView.folder',
    load: async (ids: string[]) => await db.select().from(Folders).where(inArray(Folders.entityId, ids)),
    key: ({ entityId }: { entityId: string }) => entityId,
  });

type PublishedRow = Awaited<ReturnType<typeof buildPublishedPublicationsByEntityIdsQuery>>[number];
type SiteEntry = typeof Entities.$inferSelect | PublishedRow;

export const resolveEntries = async (ctx: Context, tree: SiteTree, parentId: string | null): Promise<SiteEntry[]> => {
  const rows = visibleChildren(tree, parentId);
  const documentEntityIds = rows.filter((row) => row.type === EntityType.DOCUMENT).map((row) => row.id);
  const folderEntityIds = rows.filter((row) => row.type === EntityType.FOLDER).map((row) => row.id);
  const [publications, folders] = await Promise.all([
    documentEntityIds.length > 0 ? buildPublishedPublicationsByEntityIdsQuery(db, { entityIds: documentEntityIds }) : [],
    folderEntityIds.length > 0 ? SiteFolderView.getDataloader(ctx).loadMany(folderEntityIds) : [],
  ]);
  const publicationByEntity = new Map(publications.map((row) => [row.entityId, row]));
  const folderByEntity = new Map<string, typeof Entities.$inferSelect>();
  for (const folder of folders) {
    if (folder instanceof Error) throw folder;
    folderByEntity.set(folder.id, folder);
  }
  const entries: SiteEntry[] = [];
  for (const row of rows) {
    const entry = row.type === EntityType.FOLDER ? folderByEntity.get(row.id) : publicationByEntity.get(row.id);
    if (entry) entries.push(entry);
  }
  return entries;
};

const loadPage = async (input: {
  siteId: string;
  tagName?: string;
  excludePinned?: boolean;
  first?: number | null;
  after?: string | null;
}) => {
  const limit = clampPageSize(input.first);
  const rows = await buildPublishedPublicationsQuery(db, {
    siteId: input.siteId,
    tagName: input.tagName,
    excludePinned: input.excludePinned,
    after: input.after ?? null,
    limit: limit + 1,
  });
  return toPublicationsPage(rows, limit);
};

const SiteTag = builder.objectRef<{ siteId: string; name: string; count: number }>('SiteTag').implement({
  fields: (t) => ({
    name: t.exposeString('name'),
    count: t.exposeInt('count'),
    publications: t.field({
      type: SitePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (self, args) => await loadPage({ siteId: self.siteId, tagName: self.name, first: args.first, after: args.after }),
    }),
  }),
});

export const SiteEntryView = builder.unionType('SiteEntryView', {
  types: [PublicationView, SiteFolderView],
  resolveType: (value) => (isTypeOf(TableCode.PUBLICATIONS)(value) ? PublicationView : SiteFolderView),
});

SiteFolderView.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  fields: (t) => ({
    id: t.exposeID('id'),
    number: t.exposeString('number'),
    name: t.string({
      resolve: async (self, _, ctx) => {
        const folder = await folderLoader(ctx).load(self.id);
        return folder.name;
      },
    }),
    thumbnail: t.field({
      type: Image,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const folder = await folderLoader(ctx).load(self.id);
        return folder.thumbnailId;
      },
    }),
    url: t.string({
      resolve: async (self, _, ctx) => {
        const site = await siteLoader(ctx).load(self.siteId);
        return folderUrl(env.USERSITE_URL, site.slug, self.number);
      },
    }),
    site: t.expose('siteId', { type: SiteView }),
    children: t.field({
      type: [SiteEntryView],
      resolve: async (self, _, ctx) => {
        const { tree } = await siteTreeLoader(ctx).load(self.siteId);
        return await resolveEntries(ctx, tree, self.id);
      },
    }),
    ancestors: t.field({
      type: [SiteFolderView],
      resolve: async (self, _, ctx) => {
        const { tree } = await siteTreeLoader(ctx).load(self.siteId);
        return visibleAncestors(tree, self.id).map((row) => row.id);
      },
    }),
    folderCount: t.int({
      resolve: async (self, _, ctx) => {
        const { tree } = await siteTreeLoader(ctx).load(self.siteId);
        return visibleChildCounts(tree, self.id).folders;
      },
    }),
    publicationCount: t.int({
      resolve: async (self, _, ctx) => {
        const { tree } = await siteTreeLoader(ctx).load(self.siteId);
        return visibleChildCounts(tree, self.id).documents;
      },
    }),
  }),
});

SiteView.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  interfaces: [ISite],
  fields: (t) => ({
    availableActions: t.field({
      type: [SiteAvailableAction],
      resolve: async (self, _, ctx) => {
        const owner = await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id })
          .then(() => true)
          .catch(() => false);
        return owner ? [SiteAvailableAction.SETTINGS] : [];
      },
    }),
    pinnedPublications: t.field({
      type: [PublicationView],
      resolve: async (self) => await buildPinnedPublicationsQuery(db, { siteIds: [self.id] }),
    }),
    publicationCount: t.int({
      resolve: async (self) => await buildPublishedPublicationCountQuery(db, { siteId: self.id }).then((rows) => rows[0]?.count ?? 0),
    }),
    entries: t.field({
      type: [SiteEntryView],
      resolve: async (self, _, ctx) => {
        const { tree } = await siteTreeLoader(ctx).load(self.id);
        return await resolveEntries(ctx, tree, null);
      },
    }),
    recentPublications: t.field({
      type: SitePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (self, args) => await loadPage({ siteId: self.id, excludePinned: true, first: args.first, after: args.after }),
    }),
    publication: t.field({
      type: PublicationView,
      args: { number: t.arg.string() },
      resolve: async (self, args) =>
        await buildPublishedPublicationsQuery(db, { siteId: self.id, number: args.number, after: null, limit: 1 }).then(
          firstOrThrowWith(new NotFoundError()),
        ),
    }),
    folder: t.field({
      type: SiteFolderView,
      args: { number: t.arg.string() },
      resolve: async (self, args, ctx) => {
        const entity = await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(and(eq(Entities.siteId, self.id), eq(Entities.number, args.number), eq(Entities.type, EntityType.FOLDER)))
          .then(firstOrThrowWith(new NotFoundError()));
        const { tree } = await siteTreeLoader(ctx).load(self.id);
        if (!tree.visible.has(entity.id)) throw new NotFoundError();
        return entity.id;
      },
    }),
    tags: t.field({
      type: [SiteTag],
      resolve: async (self) => {
        const rows = await buildSiteTagsQuery(db, { siteId: self.id });
        return rows.map((row) => ({ ...row, siteId: self.id }));
      },
    }),
    tag: t.field({
      type: SiteTag,
      args: { name: t.arg.string() },
      resolve: async (self, args) => {
        const name = args.name.normalize('NFC').trim();
        const found = await buildSiteTagQuery(db, { siteId: self.id, name }).then((rows) => rows[0]);
        if (!found) throw new NotFoundError();
        return { ...found, siteId: self.id };
      },
    }),
    sitemap: t.stringList({
      resolve: async (self, _, ctx) => {
        if (!self.allowIndexing) return [];
        const [{ tree }, publications, tags] = await Promise.all([
          siteTreeLoader(ctx).load(self.id),
          buildPublishedPublicationNumbersQuery(db, { siteId: self.id, limit: 5000 }),
          buildSiteTagsQuery(db, { siteId: self.id }),
        ]);
        const folderIds = visibleFolderIds(tree);
        const folders =
          folderIds.length > 0 ? await db.select({ number: Entities.number }).from(Entities).where(inArray(Entities.id, folderIds)) : [];
        return buildSitemapPaths({
          publicationNumbers: publications.map((row) => row.number),
          folderNumbers: folders.map((row) => row.number),
          tagNames: tags.map((row) => row.name),
        });
      },
    }),
  }),
});

builder.queryFields((t) => ({
  siteView: t.field({
    type: SiteView,
    args: { slug: t.arg.string() },
    resolve: async (_, args) => await buildSiteBySlugQuery(db, { slug: args.slug }).then(firstOrThrowWith(new NotFoundError())),
  }),

  sitemapSiteSlugs: t.stringList({
    resolve: async () => {
      const rows = await buildIndexableSiteSlugsQuery(db);
      return rows.map((row) => row.slug);
    },
  }),
}));

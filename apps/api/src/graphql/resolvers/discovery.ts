import { inArray } from 'drizzle-orm';
import { db, Documents, TableCode, validateDbId } from '#/db/index.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import { getAllDiscoveryTags, getDiscoveryTags } from '#/utils/discovery.ts';
import {
  buildDiscoverablePublicationsByIdsQuery,
  buildDiscoverableSitesByIdsQuery,
  buildDiscoveryFeedQuery,
  buildDiscoveryPublicationsQuery,
  buildDiscoveryRecentPublicationsQuery,
  buildDiscoveryTagCountsQuery,
  buildDiscoveryTagsQuery,
  DISCOVERY_RECENT_SCAN_LIMIT,
  pickFirstByKey,
} from '#/utils/discovery-core.ts';
import {
  buildPublicationSearchRequest,
  buildSpaceSearchRequest,
  buildTagSearchRequest,
  filterHitsByIds,
  normalizeSearchQuery,
} from '#/utils/discovery-search-core.ts';
import { checkDocumentViewAccess, RESTRICTED_EXCERPT } from '#/utils/document-view-access.ts';
import { clampPageSize, toPublicationsPage } from '#/utils/publication-view-core.ts';
import { sanitizeHighlight } from '#/utils/search-highlight.ts';
import { pathAncestors } from '#/utils/site-tree-core.ts';
import { decompose } from '#/utils/text.ts';
import { builder } from '../builder.ts';
import { PublicationView, SiteFolderView, SiteView } from '../objects.ts';
import { SitePublicationsPage, siteTreeLoader } from './site-view.ts';
import type { SiteTree } from '#/utils/site-tree-core.ts';

const loadPage = async (input: { tagName?: string; first?: number | null; after?: string | null }) => {
  const limit = clampPageSize(input.first);
  const rows = await buildDiscoveryPublicationsQuery(db, {
    tagName: input.tagName,
    after: input.after ?? null,
    limit: limit + 1,
  });
  return toPublicationsPage(rows, limit);
};

const DiscoveryTag = builder.objectRef<{ name: string; count: number }>('DiscoveryTag').implement({
  fields: (t) => ({
    name: t.exposeString('name'),
    count: t.exposeInt('count'),
    publications: t.field({
      type: SitePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (self, args) => await loadPage({ tagName: self.name, first: args.first, after: args.after }),
    }),
  }),
});

const DiscoveryRecentSite = builder.objectRef<{ siteId: string; publicationId: string }>('DiscoveryRecentSite').implement({
  fields: (t) => ({
    site: t.expose('siteId', { type: SiteView }),
    publication: t.expose('publicationId', { type: PublicationView }),
  }),
});

const DiscoveryRecentSeries = builder.objectRef<{ folderEntityId: string; publicationId: string }>('DiscoveryRecentSeries').implement({
  fields: (t) => ({
    folder: t.expose('folderEntityId', { type: SiteFolderView }),
    publication: t.expose('publicationId', { type: PublicationView }),
  }),
});

const DISCOVERY_RECENT_LIMIT_MAX = 10;

type RecentPublicationRow = { id: string; siteId: string; documentId: string };

const loadRecentPublications = () => {
  let rows: Promise<RecentPublicationRow[]> | undefined;
  return async () => await (rows ??= buildDiscoveryRecentPublicationsQuery(db, { limit: DISCOVERY_RECENT_SCAN_LIMIT }).execute());
};

const clampRecentLimit = (first: number) => Math.min(Math.max(first, 1), DISCOVERY_RECENT_LIMIT_MAX);

const DiscoveryPublicationHit = builder
  .objectRef<{ publicationId: string; title: string | null; excerpt: string | null }>('DiscoveryPublicationHit')
  .implement({
    fields: (t) => ({
      publication: t.expose('publicationId', { type: PublicationView }),
      title: t.exposeString('title', { nullable: true }),
      excerpt: t.exposeString('excerpt', { nullable: true }),
    }),
  });

const DiscoverySearchResult = builder.simpleObject('DiscoverySearchResult', {
  fields: (t) => ({
    publications: t.field({ type: [DiscoveryPublicationHit] }),
    sites: t.field({ type: [SiteView] }),
    tags: t.field({ type: [DiscoveryTag] }),
  }),
});

const emptySearchResult = { publications: [], sites: [], tags: [] };

type Highlight = Record<string, string[] | undefined> | undefined;

const DiscoveryView = builder.objectRef<{ recentPublications: () => Promise<RecentPublicationRow[]> }>('DiscoveryView').implement({
  fields: (t) => ({
    publications: t.field({
      type: SitePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (_, args) => await loadPage({ first: args.first, after: args.after }),
    }),
    feed: t.field({
      type: SitePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (_, args) => {
        const limit = clampPageSize(args.first);
        const rows = await buildDiscoveryFeedQuery(db, { after: args.after ?? null, limit: limit + 1 });
        return toPublicationsPage(rows, limit);
      },
    }),
    tags: t.field({
      type: [DiscoveryTag],
      resolve: async () => await getDiscoveryTags(),
    }),
    allTags: t.field({
      type: [DiscoveryTag],
      resolve: async () => await getAllDiscoveryTags(),
    }),
    recentSites: t.field({
      type: [DiscoveryRecentSite],
      args: { first: t.arg.int({ defaultValue: 5 }) },
      resolve: async (self, args) => {
        const rows = await self.recentPublications();
        return pickFirstByKey(rows, (row) => row.siteId, clampRecentLimit(args.first)).map((row) => ({
          siteId: row.siteId,
          publicationId: row.id,
        }));
      },
    }),
    recentSeries: t.field({
      type: [DiscoveryRecentSeries],
      args: { first: t.arg.int({ defaultValue: 4 }) },
      resolve: async (self, args, ctx) => {
        const rows = await self.recentPublications();
        if (rows.length === 0) return [];
        const documents = await db
          .select({ id: Documents.id, entityId: Documents.entityId })
          .from(Documents)
          .where(
            inArray(
              Documents.id,
              rows.map((row) => row.documentId),
            ),
          );
        const entityByDocument = new Map(documents.map((row) => [row.id, row.entityId]));
        const trees = await siteTreeLoader(ctx).loadMany([...new Set(rows.map((row) => row.siteId))]);
        const treeBySite = new Map<string, SiteTree>();
        for (const loaded of trees) {
          if (loaded instanceof Error) throw loaded;
          treeBySite.set(loaded.id, loaded.tree);
        }
        const withFolder = rows.flatMap((row) => {
          const entityId = entityByDocument.get(row.documentId);
          const tree = treeBySite.get(row.siteId);
          if (!entityId || !tree) return [];
          const folder = pathAncestors(tree, entityId).at(-1);
          return folder ? [{ folderEntityId: folder.id, publicationId: row.id }] : [];
        });
        return pickFirstByKey(withFolder, (row) => row.folderEntityId, clampRecentLimit(args.first));
      },
    }),
    tag: t.field({
      type: DiscoveryTag,
      nullable: true,
      args: { name: t.arg.string() },
      resolve: async (_, args) => {
        const name = args.name.normalize('NFC').trim();
        if (!name) return null;
        const found = await buildDiscoveryTagsQuery(db, { name, limit: 1 }).then((rows) => rows[0]);
        return found ?? null;
      },
    }),
    search: t.field({
      type: DiscoverySearchResult,
      nullable: true,
      args: { query: t.arg.string() },
      resolve: async (_, args, ctx) => {
        const query = normalizeSearchQuery(args.query);
        if (!query) return emptySearchResult;
        const decomposedQuery = decompose(query);

        const [publicationResult, siteResult, tagResult] = await Promise.all([
          elasticsearch.search(buildPublicationSearchRequest({ index: esIndex.publications, query, decomposedQuery })),
          elasticsearch.search(buildSpaceSearchRequest({ index: esIndex.sites, query, decomposedQuery })),
          elasticsearch.search(buildTagSearchRequest({ index: esIndex.tags, query, decomposedQuery })),
        ]);

        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const publicationHits = publicationResult.hits.hits.map((hit) => ({ id: hit._id!, highlight: hit.highlight as Highlight }));
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const siteHits = siteResult.hits.hits.map((hit) => ({ id: hit._id! }));
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const tagHits = tagResult.hits.hits.map((hit) => ({ id: hit._id! }));

        const [publications, sites, tagCounts] = await Promise.all([
          publicationHits.length > 0
            ? buildDiscoverablePublicationsByIdsQuery(db, { publicationIds: publicationHits.map((hit) => hit.id) })
            : [],
          siteHits.length > 0 ? buildDiscoverableSitesByIdsQuery(db, { siteIds: siteHits.map((hit) => hit.id) }) : [],
          tagHits.length > 0 ? buildDiscoveryTagCountsQuery(db, { names: tagHits.map((hit) => hit.id) }) : [],
        ]);

        const publicationById = new Map(publications.map((row) => [row.id, row]));
        const visiblePublicationHits = filterHitsByIds(publicationHits, publicationById.keys());
        const documents =
          visiblePublicationHits.length > 0
            ? await db
                .select({ id: Documents.id, contentRating: Documents.contentRating, password: Documents.password })
                .from(Documents)
                .where(
                  inArray(
                    Documents.id,
                    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                    visiblePublicationHits.map((hit) => publicationById.get(hit.id)!.documentId),
                  ),
                )
            : [];
        const documentById = new Map(documents.map((row) => [row.id, row]));

        const publicationEntries = await Promise.all(
          visiblePublicationHits.map(async (hit) => {
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            const publication = publicationById.get(hit.id)!;
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            const document = documentById.get(publication.documentId)!;
            const access = await checkDocumentViewAccess(document, ctx);
            return {
              publicationId: hit.id,
              title: sanitizeHighlight(hit.highlight?.title?.[0]) ?? null,
              excerpt: access.accessible ? (sanitizeHighlight(hit.highlight?.text?.[0]) ?? null) : RESTRICTED_EXCERPT,
            };
          }),
        );

        const siteById = new Map(sites.map((row) => [row.id, row]));
        const countByName = new Map(tagCounts.map((row) => [row.name, row.count]));

        return {
          publications: publicationEntries,
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          sites: filterHitsByIds(siteHits, siteById.keys()).map((hit) => siteById.get(hit.id)!),
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          tags: filterHitsByIds(tagHits, countByName.keys()).map((hit) => ({ name: hit.id, count: countByName.get(hit.id)! })),
        };
      },
    }),
  }),
});

builder.queryFields((t) => ({
  discovery: t.field({
    type: DiscoveryView,
    resolve: () => ({ recentPublications: loadRecentPublications() }),
  }),
}));

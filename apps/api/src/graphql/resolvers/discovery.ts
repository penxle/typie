import { inArray } from 'drizzle-orm';
import { db, Documents, TableCode, validateDbId } from '#/db/index.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import { getDiscoveryTags } from '#/utils/discovery.ts';
import {
  buildDiscoverablePublicationsByIdsQuery,
  buildDiscoverableSpacesByIdsQuery,
  buildDiscoveryPublicationsQuery,
  buildDiscoveryTagCountsQuery,
  buildDiscoveryTagsQuery,
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
import { decompose } from '#/utils/text.ts';
import { builder } from '../builder.ts';
import { PublicationView, SpaceView } from '../objects.ts';
import { SpacePublicationsPage } from './space-view.ts';

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
      type: SpacePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (self, args) => await loadPage({ tagName: self.name, first: args.first, after: args.after }),
    }),
  }),
});

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
    spaces: t.field({ type: [SpaceView] }),
    tags: t.field({ type: [DiscoveryTag] }),
  }),
});

const emptySearchResult = { publications: [], spaces: [], tags: [] };

type Highlight = Record<string, string[] | undefined> | undefined;

const DiscoveryView = builder.objectRef<Record<string, never>>('DiscoveryView').implement({
  fields: (t) => ({
    publications: t.field({
      type: SpacePublicationsPage,
      args: {
        first: t.arg.int({ required: false }),
        after: t.arg.id({ required: false, validate: validateDbId(TableCode.PUBLICATIONS) }),
      },
      resolve: async (_, args) => await loadPage({ first: args.first, after: args.after }),
    }),
    tags: t.field({
      type: [DiscoveryTag],
      resolve: async () => await getDiscoveryTags(),
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

        const [publicationResult, spaceResult, tagResult] = await Promise.all([
          elasticsearch.search(buildPublicationSearchRequest({ index: esIndex.publications, query, decomposedQuery })),
          elasticsearch.search(buildSpaceSearchRequest({ index: esIndex.spaces, query, decomposedQuery })),
          elasticsearch.search(buildTagSearchRequest({ index: esIndex.tags, query, decomposedQuery })),
        ]);

        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const publicationHits = publicationResult.hits.hits.map((hit) => ({ id: hit._id!, highlight: hit.highlight as Highlight }));
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const spaceHits = spaceResult.hits.hits.map((hit) => ({ id: hit._id! }));
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const tagHits = tagResult.hits.hits.map((hit) => ({ id: hit._id! }));

        const [publications, spaces, tagCounts] = await Promise.all([
          publicationHits.length > 0
            ? buildDiscoverablePublicationsByIdsQuery(db, { publicationIds: publicationHits.map((hit) => hit.id) })
            : [],
          spaceHits.length > 0 ? buildDiscoverableSpacesByIdsQuery(db, { spaceIds: spaceHits.map((hit) => hit.id) }) : [],
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

        const spaceById = new Map(spaces.map((row) => [row.id, row]));
        const countByName = new Map(tagCounts.map((row) => [row.name, row.count]));

        return {
          publications: publicationEntries,
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          spaces: filterHitsByIds(spaceHits, spaceById.keys()).map((hit) => spaceById.get(hit.id)!),
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
    resolve: () => ({}),
  }),
}));

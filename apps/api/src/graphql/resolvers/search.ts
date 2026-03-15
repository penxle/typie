import { SearchHitType } from '@typie/lib/enums';
import DOMPurify from 'isomorphic-dompurify';
import { match } from 'ts-pattern';
import { TableCode, validateDbId } from '#/db/index.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { decompose } from '#/utils/text.ts';
import { builder } from '../builder.ts';
import { Document, Folder } from '../objects.ts';

/**
 * * Types
 */

const SearchHitDocument = builder.simpleObject('SearchHitDocument', {
  fields: (t) => ({
    type: t.field({ type: SearchHitType }),
    title: t.string({ nullable: true }),
    subtitle: t.string({ nullable: true }),
    text: t.string({ nullable: true }),
    document: t.field({ type: Document }),
  }),
});

const SearchHitFolder = builder.simpleObject('SearchHitFolder', {
  fields: (t) => ({
    type: t.field({ type: SearchHitType }),
    name: t.string({ nullable: true }),
    folder: t.field({ type: Folder }),
  }),
});

const SearchHit = builder.unionType('SearchHit', {
  types: [SearchHitDocument, SearchHitFolder],
  resolveType: (self) =>
    match(self.type)
      .with(SearchHitType.DOCUMENT, () => 'SearchHitDocument')
      .with(SearchHitType.FOLDER, () => 'SearchHitFolder')
      .exhaustive(),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  search: t.withAuth({ session: true }).field({
    type: builder.simpleObject('SearchResult', {
      fields: (t) => ({
        totalHits: t.int(),
        documentCount: t.int(),
        folderCount: t.int(),
        hits: t.field({
          type: [SearchHit],
        }),
      }),
    }),
    args: {
      siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }),
      query: t.arg.string(),
      ancestorEntityId: t.arg.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, args, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: args.siteId,
      });

      const trimmedQuery = args.query.trim();

      if (!trimmedQuery) {
        return {
          totalHits: 0,
          documentCount: 0,
          folderCount: 0,
          hits: [],
        };
      }

      const decomposedQuery = decompose(trimmedQuery);

      const result = await elasticsearch.search({
        index: [esIndex.documents, esIndex.folders],
        size: 20,
        _source: false,
        query: {
          bool: {
            should: [
              { match: { title: { query: trimmedQuery, boost: 3 } } },
              { match: { subtitle: { query: trimmedQuery, boost: 2 } } },
              { match: { text: { query: trimmedQuery } } },
              { match: { name: { query: trimmedQuery, boost: 2 } } },
              ...(decomposedQuery
                ? [
                    { match: { title_decomposed: { query: decomposedQuery, boost: 1.5 } } },
                    { match: { subtitle_decomposed: { query: decomposedQuery, boost: 1 } } },
                    { match: { name_decomposed: { query: decomposedQuery } } },
                  ]
                : []),
            ],
            filter: [
              { term: { site_id: args.siteId } },
              ...(args.ancestorEntityId ? [{ term: { ancestor_ids: args.ancestorEntityId } }] : []),
            ],
            minimum_should_match: 1,
          },
        },
        highlight: {
          fields: {
            title: {},
            subtitle: {},
            text: { fragment_size: 200, number_of_fragments: 1 },
            name: {},
          },
          pre_tags: ['<em>'],
          post_tags: ['</em>'],
        },
        aggs: {
          by_index: { terms: { field: '_index' } },
        },
      });

      const buckets = (result.aggregations?.by_index as { buckets: { key: string; doc_count: number }[] })?.buckets ?? [];
      const documentCount = buckets.find((b) => b.key === esIndex.documents)?.doc_count ?? 0;
      const folderCount = buckets.find((b) => b.key === esIndex.folders)?.doc_count ?? 0;

      const hits = result.hits.hits.map((hit) => {
        if (hit._index === esIndex.documents) {
          return {
            type: SearchHitType.DOCUMENT,
            title: sanitizeHtml(hit.highlight?.title?.[0]),
            subtitle: sanitizeHtml(hit.highlight?.subtitle?.[0]),
            text: sanitizeHtml(hit.highlight?.text?.[0]),
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            document: hit._id!,
          };
        }

        return {
          type: SearchHitType.FOLDER,
          name: sanitizeHtml(hit.highlight?.name?.[0]),
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          folder: hit._id!,
        };
      });

      return {
        totalHits: documentCount + folderCount,
        documentCount,
        folderCount,
        hits,
      };
    },
  }),
}));

/**
 * * Utils
 */

const sanitizeHtml = (dirty: string | undefined) => {
  return dirty ? DOMPurify.sanitize(dirty, { ALLOWED_TAGS: ['em'] }) : undefined;
};

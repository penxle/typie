import DOMPurify from 'isomorphic-dompurify';
import { match } from 'ts-pattern';
import { TableCode, validateDbId } from '@/db';
import { SearchHitType } from '@/enums';
import { meilisearch } from '@/search';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Document, Folder, Post } from '../objects';

/**
 * * Types
 */

const SearchHitPost = builder.simpleObject('SearchHitPost', {
  fields: (t) => ({
    type: t.field({ type: SearchHitType }),
    title: t.string({ nullable: true }),
    subtitle: t.string({ nullable: true }),
    text: t.string({ nullable: true }),
    post: t.field({ type: Post }),
  }),
});

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
  types: [SearchHitPost, SearchHitDocument, SearchHitFolder],
  resolveType: (self) =>
    match(self.type)
      .with(SearchHitType.DOCUMENT, () => 'SearchHitDocument')
      .with(SearchHitType.POST, () => 'SearchHitPost')
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
        hits: t.field({
          type: [SearchHit],
        }),
      }),
    }),
    args: {
      siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }),
      query: t.arg.string(),
    },
    resolve: async (_, args, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: args.siteId,
      });

      if (!args.query.trim()) {
        return {
          totalHits: 0,
          hits: [],
        };
      }

      const result = await meilisearch.multiSearch({
        federation: {},
        queries: [
          {
            indexUid: 'documents',
            q: args.query.trim(),
            filter: [`siteId = ${args.siteId}`],
            attributesToCrop: ['*'],
            attributesToHighlight: ['title', 'subtitle', 'text'],
          },
          {
            indexUid: 'folders',
            q: args.query.trim(),
            filter: [`siteId = ${args.siteId}`],
            attributesToHighlight: ['name'],
          },
        ],
      });

      return {
        totalHits: result.estimatedTotalHits ?? 0,
        hits: result.hits.map((hit) => {
          const indexUid = hit._federation?.indexUid;

          if (indexUid === 'documents') {
            return {
              type: SearchHitType.DOCUMENT,
              title: sanitizeHtml(hit._formatted?.title),
              subtitle: sanitizeHtml(hit._formatted?.subtitle),
              text: sanitizeHtml(hit._formatted?.text),
              document: hit.id,
            };
          }

          if (indexUid === 'folders') {
            return {
              type: SearchHitType.FOLDER,
              name: sanitizeHtml(hit._formatted?.name),
              folder: hit.id,
            };
          }

          throw new Error(`Unknown index: ${indexUid}`);
        }),
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

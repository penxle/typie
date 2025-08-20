import DOMPurify from 'isomorphic-dompurify';
import { match } from 'ts-pattern';
import { TableCode, validateDbId } from '@/db';
import { SearchHitType } from '@/enums';
import { meilisearch } from '@/search';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Canvas, Post } from '../objects';

/**
 * * Queries
 */

builder.queryFields((t) => ({
  search: t.withAuth({ session: true }).field({
    type: builder.simpleObject('SearchResult', {
      fields: (t) => ({
        totalHits: t.int(),
        hits: t.field({
          type: [
            builder.unionType('SearchHit', {
              types: [
                builder.simpleObject('SearchHitPost', {
                  fields: (t) => ({
                    type: t.field({ type: SearchHitType }),
                    title: t.string({ nullable: true }),
                    subtitle: t.string({ nullable: true }),
                    text: t.string({ nullable: true }),
                    post: t.field({ type: Post }),
                  }),
                }),
                builder.simpleObject('SearchHitCanvas', {
                  fields: (t) => ({
                    type: t.field({ type: SearchHitType }),
                    title: t.string({ nullable: true }),
                    canvas: t.field({ type: Canvas }),
                  }),
                }),
              ],
              resolveType: (self) =>
                match(self.type)
                  .with(SearchHitType.POST, () => 'SearchHitPost')
                  .with(SearchHitType.CANVAS, () => 'SearchHitCanvas')
                  .exhaustive(),
            }),
          ],
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
            indexUid: 'posts',
            q: args.query.trim(),
            filter: [`siteId = ${args.siteId}`],
            attributesToCrop: ['*'],
            attributesToHighlight: ['title', 'subtitle', 'text'],
          },
          {
            indexUid: 'canvases',
            q: args.query.trim(),
            filter: [`siteId = ${args.siteId}`],
            attributesToCrop: ['*'],
            attributesToHighlight: ['title'],
          },
        ],
      });

      return {
        totalHits: result.estimatedTotalHits ?? 0,
        hits: result.hits.map((hit) =>
          match(hit._federation?.indexUid)
            .with('posts', () => ({
              type: SearchHitType.POST,
              title: sanitizeHtml(hit._formatted?.title),
              subtitle: sanitizeHtml(hit._formatted?.subtitle),
              text: sanitizeHtml(hit._formatted?.text),
              post: hit.id,
            }))
            .with('canvases', () => ({
              type: SearchHitType.CANVAS,
              title: sanitizeHtml(hit._formatted?.title),
              canvas: hit.id,
            }))
            .run(),
        ),
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

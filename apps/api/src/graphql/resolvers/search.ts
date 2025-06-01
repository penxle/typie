import DOMPurify from 'isomorphic-dompurify';
import { match } from 'ts-pattern';
import { TableCode, validateDbId } from '@/db';
import { SearchHitType } from '@/enums';
import { elastic } from '@/search';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Post } from '../objects';

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
              ],
              resolveType: (self) =>
                match(self.type)
                  .with(SearchHitType.POST, () => 'SearchHitPost')
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

      const result = await elastic.search<{
        id: string;
        siteId: string;
        title?: string;
        subtitle?: string;
        text?: string;
        updatedAt: number;
      }>({
        index: 'posts',
        query: {
          bool: {
            must: [
              {
                multi_match: {
                  query: args.query,
                  fields: ['title^2', 'subtitle^1.5', 'text'],
                  type: 'best_fields',
                  analyzer: 'nori',
                },
              },
            ],
            filter: [
              {
                term: {
                  siteId: args.siteId,
                },
              },
            ],
          },
        },
        highlight: {
          fields: {
            title: { fragment_size: 200 },
            subtitle: { fragment_size: 200 },
            text: { fragment_size: 200 },
          },
          pre_tags: ['<em>'],
          post_tags: ['</em>'],
        },
      });

      return {
        totalHits: result.hits.total ? (typeof result.hits.total === 'number' ? result.hits.total : result.hits.total.value) : 0,
        hits: result.hits.hits.map((hit) => ({
          type: SearchHitType.POST,
          title: sanitizeHtml(hit.highlight?.title?.[0] || hit._source?.title) ?? null,
          subtitle: sanitizeHtml(hit.highlight?.subtitle?.[0] || hit._source?.subtitle) ?? null,
          text: sanitizeHtml(hit.highlight?.text?.[0] || hit._source?.text) ?? null,
          post: hit._id || '',
        })),
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

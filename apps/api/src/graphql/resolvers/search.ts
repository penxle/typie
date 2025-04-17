import DOMPurify from 'isomorphic-dompurify';
import { match } from 'ts-pattern';
import { TableCode, validateDbId } from '@/db';
import { SearchHitType } from '@/enums';
import { meili } from '@/search';
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

      const result = await meili.multiSearch({
        federation: {},
        queries: [
          {
            indexUid: 'posts',
            q: args.query,
            filter: [`siteId = ${args.siteId}`],
            attributesToCrop: ['*'],
            attributesToHighlight: ['title', 'subtitle', 'text'],
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

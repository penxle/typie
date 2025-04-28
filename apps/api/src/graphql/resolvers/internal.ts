import { eq } from 'drizzle-orm';
import * as Y from 'yjs';
import { db, first, PostContents } from '@/db';
import { schema } from '@/pm';
import { makeYDoc } from '@/utils';
import { builder } from '../builder';

/**
 * * Queries
 */

builder.queryFields((t) => ({
  seed: t.field({
    type: 'Float',
    resolve: () => {
      return Math.random();
    },
  }),

  welcome: t.field({
    type: builder.simpleObject('Welcome', {
      fields: (t) => ({
        body: t.field({ type: 'JSON' }),
        update: t.field({ type: 'Binary' }),
      }),
    }),
    resolve: async () => {
      const content = await db
        .select({ body: PostContents.body })
        .from(PostContents)
        .where(eq(PostContents.postId, 'P0WELCOME'))
        .then(first);

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const body = content?.body ?? schema.topNodeType.createAndFill()!.toJSON();

      const yDoc = makeYDoc({ body });
      const update = Y.encodeStateAsUpdateV2(yDoc);

      return {
        body,
        update,
      };
    },
  }),
}));

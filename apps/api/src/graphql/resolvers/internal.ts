import { eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import * as Y from 'yjs';
import { db, first, PostContents } from '@/db';
import { schema } from '@/pm';
import { generateRandomName, makeYDoc } from '@/utils';
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

  randomName: t.field({
    type: 'String',
    resolve: () => {
      return generateRandomName(nanoid());
    },
  }),

  welcome: t.field({
    type: builder.simpleObject('Welcome', {
      fields: (t) => ({
        body: t.field({ type: 'JSON' }),
        update: t.field({ type: 'Binary' }),
        name: t.string(),
        bodyMobile: t.field({ type: 'JSON' }),
        updateMobile: t.field({ type: 'Binary' }),
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

      const name = generateRandomName(nanoid());

      const contentMobile = await db
        .select({ body: PostContents.body })
        .from(PostContents)
        .where(eq(PostContents.postId, 'P0WELCOMEMOBILE'))
        .then(first);

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const bodyMobile = contentMobile?.body ?? schema.topNodeType.createAndFill()!.toJSON();

      const yDocMobile = makeYDoc({ body: bodyMobile });
      const updateMobile = Y.encodeStateAsUpdateV2(yDocMobile);

      return {
        body,
        update,
        name,
        bodyMobile,
        updateMobile,
      };
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  generateRandomName: t.field({
    type: 'String',
    resolve: () => {
      return generateRandomName(nanoid());
    },
  }),
}));

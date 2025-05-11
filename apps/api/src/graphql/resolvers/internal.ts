import { and, asc, eq, getTableColumns } from 'drizzle-orm';
import * as Y from 'yjs';
import { db, Entities, first, Folders, PostContents, Posts } from '@/db';
import { EntityState, EntityVisibility } from '@/enums';
import { schema } from '@/pm';
import { generateRandomName, makeYDoc } from '@/utils';
import { builder } from '../builder';
import { PostView } from '../objects';

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
      return generateRandomName();
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

      const name = generateRandomName();

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

  announcements: t.field({
    type: [PostView],
    resolve: async () => {
      const folder = await db.select({ entityId: Folders.entityId }).from(Folders).where(eq(Folders.id, 'F0ANNOUNCEMENTS')).then(first);
      if (!folder) {
        return [];
      }

      return await db
        .select(getTableColumns(Posts))
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(
          and(
            eq(Entities.parentId, folder.entityId),
            eq(Entities.state, EntityState.ACTIVE),
            eq(Entities.visibility, EntityVisibility.UNLISTED),
          ),
        )
        .orderBy(asc(Entities.order));
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
      return generateRandomName();
    },
  }),
}));

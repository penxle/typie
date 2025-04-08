import { faker } from '@faker-js/faker';
import { and, desc, eq, getTableColumns, isNull } from 'drizzle-orm';
import { generateJitteredKeyBetween } from 'fractional-indexing-jittered';
import * as Y from 'yjs';
import { db, Entities, first, firstOrThrow, PostContents, PostContentSnapshots, PostOptions, Posts, TableCode } from '@/db';
import { EntityState, EntityType, PostVisibility } from '@/enums';
import { schema } from '@/pm';
import { decode, encode, makeText, makeYDoc } from '@/utils';
import { builder } from '../builder';
import {
  Entity,
  EntityView,
  Image,
  IPost,
  IPostContent,
  IPostOption,
  isTypeOf,
  Post,
  PostContent,
  PostContentView,
  PostOption,
  PostOptionView,
  PostView,
} from '../objects';

/**
 * * Types
 */

IPost.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
  }),
});

Post.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    entity: t.field({ type: Entity, resolve: (self) => self.entityId }),

    content: t.field({
      type: PostContent,
      resolve: async (self) => {
        return await db.select().from(PostContents).where(eq(PostContents.postId, self.id)).then(firstOrThrow);
      },
    }),

    option: t.field({
      type: PostOption,
      resolve: async (self) => {
        return await db.select().from(PostOptions).where(eq(PostOptions.postId, self.id)).then(firstOrThrow);
      },
    }),
  }),
});

PostView.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    entity: t.field({ type: EntityView, resolve: (self) => self.entityId }),

    content: t.field({
      type: PostContentView,
      resolve: async (self) => {
        return await db.select().from(PostContents).where(eq(PostContents.postId, self.id)).then(firstOrThrow);
      },
    }),

    option: t.field({
      type: PostOptionView,
      resolve: async (self) => {
        return await db.select().from(PostOptions).where(eq(PostOptions.postId, self.id)).then(firstOrThrow);
      },
    }),
  }),
});

IPostContent.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    subtitle: t.exposeString('subtitle', { nullable: true }),

    title: t.string({ resolve: (self) => self.title || '(제목 없음)' }),

    excerpt: t.string({
      resolve: (self) => {
        const text = self.text.replaceAll(/\s+/g, ' ').trim();
        return text.length <= 200 ? text : text.slice(0, 200) + '...';
      },
    }),
  }),
});

PostContent.implement({
  isTypeOf: isTypeOf(TableCode.POST_CONTENTS),
  interfaces: [IPostContent],
  fields: (t) => ({
    update: t.expose('update', { type: 'Binary' }),
  }),
});

PostContentView.implement({
  isTypeOf: isTypeOf(TableCode.POST_CONTENTS),
  interfaces: [IPostContent],
  fields: (t) => ({
    body: t.expose('body', { type: 'JSON' }),
    maxWidth: t.exposeInt('maxWidth'),
    coverImage: t.expose('coverImageId', { type: Image, nullable: true }),
  }),
});

IPostOption.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    visibility: t.expose('visibility', { type: PostVisibility }),
    allowComments: t.exposeBoolean('allowComments'),
    allowReactions: t.exposeBoolean('allowReactions'),
    allowCopies: t.exposeBoolean('allowCopies'),
  }),
});

PostOption.implement({
  isTypeOf: isTypeOf(TableCode.POST_OPTIONS),
  interfaces: [IPostOption],
  fields: (t) => ({
    password: t.exposeString('password', { nullable: true }),
  }),
});

PostOptionView.implement({
  isTypeOf: isTypeOf(TableCode.POST_OPTIONS),
  interfaces: [IPostOption],
  fields: (t) => ({
    hasPassword: t.boolean({ resolve: (self) => !!self.password }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  post: t.withAuth({ session: true }).field({
    type: Post,
    args: { slug: t.arg.string() },
    resolve: async (_, args) => {
      return await db
        .select(getTableColumns(Posts))
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Entities.slug, args.slug))
        .then(firstOrThrow);
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createPost: t.withAuth({ session: true }).fieldWithInput({
    type: Post,
    input: {
      siteId: t.input.id(),
      parentEntityId: t.input.id({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const title = null;
      const subtitle = null;
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const node = schema.topNodeType.createAndFill()!;
      const body = node.toJSON();
      const text = makeText(body);

      const doc = makeYDoc({ title, subtitle, body });
      const snapshot = Y.snapshot(doc);

      if (input.parentEntityId) {
        await db
          .select({ id: Entities.id })
          .from(Entities)
          .where(
            and(
              eq(Entities.siteId, input.siteId),
              eq(Entities.id, input.parentEntityId),
              eq(Entities.type, EntityType.FOLDER),
              eq(Entities.state, EntityState.ACTIVE),
            ),
          )
          .then(firstOrThrow);
      }

      const last = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, input.siteId),
            input.parentEntityId ? eq(Entities.parentId, input.parentEntityId) : isNull(Entities.parentId),
          ),
        )
        .orderBy(desc(Entities.order))
        .limit(1)
        .then(first);

      return await db.transaction(async (tx) => {
        const entity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: input.siteId,
            parentId: input.parentEntityId,
            slug: faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' }),
            permalink: faker.string.alphanumeric({ length: 6, casing: 'mixed' }),
            type: EntityType.POST,
            order: encode(generateJitteredKeyBetween(last ? decode(last.order) : null, null)),
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const post = await tx
          .insert(Posts)
          .values({
            entityId: entity.id,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(PostContents).values({
          postId: post.id,
          title,
          subtitle,
          body,
          text,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });

        await tx.insert(PostContentSnapshots).values({
          postId: post.id,
          snapshot: Y.encodeSnapshotV2(snapshot),
        });

        await tx.insert(PostOptions).values({
          postId: post.id,
        });

        return post;
      });
    },
  }),

  updatePostOption: t.withAuth({ session: true }).fieldWithInput({
    type: PostOption,
    input: {
      postId: t.input.id(),
      visibility: t.input.field({ type: PostVisibility }),
      password: t.input.string(),
      allowComments: t.input.boolean(),
      allowReactions: t.input.boolean(),
      allowCopies: t.input.boolean(),
    },
    resolve: async (_, { input }) => {
      return await db
        .update(PostOptions)
        .set({
          visibility: input.visibility,
          password: input.password,
          allowComments: input.allowComments,
          allowReactions: input.allowReactions,
          allowCopies: input.allowCopies,
        })
        .where(eq(PostOptions.postId, input.postId))
        .returning()
        .then(firstOrThrow);
    },
  }),
}));

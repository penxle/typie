import { faker } from '@faker-js/faker';
import dayjs from 'dayjs';
import { and, desc, eq, getTableColumns, gte, inArray, isNull, lt, sum } from 'drizzle-orm';
import { generateJitteredKeyBetween } from 'fractional-indexing-jittered';
import * as Y from 'yjs';
import {
  Comments,
  db,
  Entities,
  first,
  firstOrThrow,
  PostCharacterCountChanges,
  PostContents,
  PostOptions,
  PostReactions,
  Posts,
  PostSnapshots,
  TableCode,
} from '@/db';
import { EntityState, EntityType, PostVisibility } from '@/enums';
import { TypieError } from '@/errors';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { decode, encode, makeText, makeYDoc } from '@/utils';
import { builder } from '../builder';
import {
  CharacterCountChange,
  Comment,
  Entity,
  EntityView,
  Image,
  IPost,
  IPostOption,
  isTypeOf,
  Post,
  PostOption,
  PostOptionView,
  PostReaction,
  PostView,
} from '../objects';

/**
 * * Types
 */

IPost.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.string({ resolve: (self) => self.title || '(제목 없음)' }),
    subtitle: t.exposeString('subtitle', { nullable: true }),
    maxWidth: t.exposeInt('maxWidth'),
    coverImage: t.expose('coverImageId', { type: Image, nullable: true }),

    excerpt: t.string({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Post.excerpt',
          load: async (ids) => {
            return await db
              .select({ postId: PostContents.postId, text: PostContents.text })
              .from(PostContents)
              .where(inArray(PostContents.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        const content = await loader.load(self.id);
        const text = content.text.replaceAll(/\s+/g, ' ').trim();

        return text.length <= 200 ? text : text.slice(0, 200) + '...';
      },
    }),
  }),
});

Post.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    update: t.field({
      type: 'Binary',
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Post.update',
          load: async (ids) => {
            return await db
              .select({ postId: PostContents.postId, update: PostContents.update })
              .from(PostContents)
              .where(inArray(PostContents.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        const content = await loader.load(self.id);

        return content.update;
      },
    }),

    entity: t.expose('entityId', { type: Entity }),

    option: t.field({
      type: PostOption,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Post.option',
          load: async (ids) => {
            return await db.select().from(PostOptions).where(inArray(PostOptions.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        return await loader.load(self.id);
      },
    }),

    characterCountChange: t.withAuth({ session: true }).field({
      type: CharacterCountChange,
      resolve: async (post, _, ctx) => {
        const startOfDay = dayjs().kst().startOf('day');

        const change = await db
          .select({
            additions: sum(PostCharacterCountChanges.additions),
            deletions: sum(PostCharacterCountChanges.deletions),
          })
          .from(PostCharacterCountChanges)
          .where(
            and(
              eq(PostCharacterCountChanges.userId, ctx.session.userId),
              eq(PostCharacterCountChanges.postId, post.id),
              gte(PostCharacterCountChanges.timestamp, startOfDay),
              lt(PostCharacterCountChanges.timestamp, startOfDay.add(1, 'day')),
            ),
          )
          .then(first);

        return {
          date: startOfDay,
          additions: Number(change?.additions ?? 0),
          deletions: Number(change?.deletions ?? 0),
        };
      },
    }),
  }),
});

PostView.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    body: t.field({
      type: 'JSON',
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'PostView.body',
          load: async (ids) => {
            return await db
              .select({ postId: PostContents.postId, body: PostContents.body })
              .from(PostContents)
              .where(inArray(PostContents.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        const content = await loader.load(self.id);

        return content.body;
      },
    }),

    entity: t.expose('entityId', { type: EntityView }),

    option: t.field({
      type: PostOptionView,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'PostView.option',
          load: async (ids) => {
            return await db.select().from(PostOptions).where(inArray(PostOptions.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        return await loader.load(self.id);
      },
    }),

    reactions: t.field({
      type: [PostReaction],
      resolve: async (post, _, ctx) => {
        const loader = ctx.loader({
          name: 'PostView.reactions',
          many: true,
          load: async (ids) => {
            return await db.select().from(PostReactions).where(inArray(PostReactions.postId, ids)).orderBy(desc(PostReactions.createdAt));
          },
          key: ({ postId }) => postId,
        });

        return await loader.load(post.id);
      },
    }),

    comments: t.field({
      type: [Comment],
      resolve: async (post, _, ctx) => {
        const optionLoader = ctx.loader({
          name: 'PostView.option',
          load: async (ids) => {
            return await db.select().from(PostOptions).where(inArray(PostOptions.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        const option = await optionLoader.load(post.id);
        if (!option.allowComments) {
          return [];
        }

        const commentLoader = ctx.loader({
          name: 'PostView.comments',
          many: true,
          load: async (ids) => {
            return await db.select().from(Comments).where(inArray(Comments.postId, ids)).orderBy(Comments.createdAt);
          },
          key: ({ postId }) => postId,
        });

        return await commentLoader.load(post.id);
      },
    }),
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

PostReaction.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    emoji: t.expose('emoji', { type: 'String' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
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

      const post = await db.transaction(async (tx) => {
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
            title,
            subtitle,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(PostContents).values({
          postId: post.id,
          body,
          text,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });

        await tx.insert(PostSnapshots).values({
          userId: ctx.session.userId,
          postId: post.id,
          snapshot: Y.encodeSnapshotV2(snapshot),
        });

        await tx.insert(PostOptions).values({
          postId: post.id,
        });

        return post;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });

      return post;
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

  createPostReaction: t.fieldWithInput({
    type: PostReaction,
    input: {
      postId: t.input.id(),
      emoji: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({
          state: Entities.state,
          allowReactions: PostOptions.allowReactions,
        })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .innerJoin(PostOptions, eq(Posts.id, PostOptions.postId))
        .where(eq(Posts.id, input.postId))
        .then(first);

      if (post?.state !== EntityState.ACTIVE) {
        throw new TypieError({ code: 'not_found' });
      }

      if (!post.allowReactions) {
        throw new TypieError({ code: 'reaction_disallowed' });
      }

      return await db
        .insert(PostReactions)
        .values({
          postId: input.postId,
          userId: ctx.session?.userId,
          emoji: input.emoji,
          deviceId: ctx.deviceId,
        })
        .returning()
        .then(firstOrThrow);
    },
  }),
}));

import { faker } from '@faker-js/faker';
import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, gte, inArray, isNull, lt, sum } from 'drizzle-orm';
import { match } from 'ts-pattern';
import * as Y from 'yjs';
import { redis } from '@/cache';
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
  UserPersonalIdentities,
  validateDbId,
} from '@/db';
import { EntityState, EntityType, PostContentRating, PostViewBodyUnavailableReason, PostVisibility } from '@/enums';
import { TypieError } from '@/errors';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { generateEntityOrder, getKoreanAge, makeText, makeYDoc } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
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
    view: t.expose('id', { type: PostView }),

    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

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
            additions: sum(PostCharacterCountChanges.additions).mapWith(Number),
            deletions: sum(PostCharacterCountChanges.deletions).mapWith(Number),
          })
          .from(PostCharacterCountChanges)
          .where(
            and(
              eq(PostCharacterCountChanges.userId, ctx.session.userId),
              eq(PostCharacterCountChanges.postId, post.id),
              gte(PostCharacterCountChanges.bucket, startOfDay),
              lt(PostCharacterCountChanges.bucket, startOfDay.add(1, 'day')),
            ),
          )
          .then(firstOrThrow);

        return {
          date: startOfDay,
          additions: change.additions ?? 0,
          deletions: change.deletions ?? 0,
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
      type: t.builder.unionType('PostViewBody', {
        types: [
          t.builder.simpleObject('PostViewBodyAvailable', {
            fields: (t) => ({ content: t.field({ type: 'JSON' }) }),
          }),
          t.builder.simpleObject('PostViewBodyUnavailable', {
            fields: (t) => ({ reason: t.field({ type: PostViewBodyUnavailableReason }) }),
          }),
        ],
      }),
      resolve: async (self, _, ctx) => {
        const optionLoader = ctx.loader({
          name: 'PostView.option',
          load: async (ids) => {
            return await db.select().from(PostOptions).where(inArray(PostOptions.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        const option = await optionLoader.load(self.id);

        if (option.contentRating !== PostContentRating.ALL) {
          if (!ctx.session) {
            return {
              __typename: 'PostViewBodyUnavailable',
              reason: PostViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION,
            };
          }

          const identity = await db
            .select({
              birthday: UserPersonalIdentities.birthDate,
              expiresAt: UserPersonalIdentities.expiresAt,
            })
            .from(UserPersonalIdentities)
            .where(eq(UserPersonalIdentities.userId, ctx.session.userId))
            .then(first);

          if (!identity) {
            return {
              __typename: 'PostViewBodyUnavailable',
              reason: PostViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION,
            };
          }

          if (identity.expiresAt.isBefore(dayjs())) {
            return {
              __typename: 'PostViewBodyUnavailable',
              reason: PostViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION,
            };
          }

          const minAge = match(option.contentRating)
            .with(PostContentRating.R15, () => 15)
            .with(PostContentRating.R19, () => 19)
            .exhaustive();

          if (getKoreanAge(identity.birthday) < minAge) {
            return {
              __typename: 'PostViewBodyUnavailable',
              reason: PostViewBodyUnavailableReason.REQUIRE_MINIMUM_AGE,
            };
          }
        }

        if (option.password !== null) {
          const passwordUnlock = await redis.get(`postview:unlock:${self.id}:${ctx.deviceId}`);

          if (passwordUnlock !== 'true') {
            return {
              __typename: 'PostViewBodyUnavailable',
              reason: PostViewBodyUnavailableReason.REQUIRE_PASSWORD,
            };
          }
        }

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

        return {
          __typename: 'PostViewBodyAvailable',
          content: content.body,
        };
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
        if (!option.allowComment) {
          return [];
        }

        const commentsLoader = ctx.loader({
          name: 'PostView.comments',
          many: true,
          load: async (ids) => {
            return await db.select().from(Comments).where(inArray(Comments.postId, ids)).orderBy(Comments.createdAt);
          },
          key: ({ postId }) => postId,
        });

        return await commentsLoader.load(post.id);
      },
    }),
  }),
});

IPostOption.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    visibility: t.expose('visibility', { type: PostVisibility }),
    contentRating: t.expose('contentRating', { type: PostContentRating }),
    allowComment: t.exposeBoolean('allowComment'),
    allowReaction: t.exposeBoolean('allowReaction'),
    protectContent: t.exposeBoolean('protectContent'),
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
  isTypeOf: isTypeOf(TableCode.POST_REACTIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    emoji: t.expose('emoji', { type: 'String' }),

    post: t.expose('postId', { type: PostView }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  post: t.withAuth({ session: true }).field({
    type: Post,
    args: { slug: t.arg.string() },
    resolve: async (_, args, ctx) => {
      const { post, entity } = await db
        .select({ post: Posts, entity: { siteId: Entities.siteId } })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Entities.slug, args.slug))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      return post;
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
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

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
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.POST,
            order: generateEntityOrder({ lower: last?.order, upper: null }),
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

  duplicatePost: t.withAuth({ session: true }).fieldWithInput({
    type: Post,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({
          siteId: Entities.siteId,
          parentEntityId: Entities.parentId,
          order: Entities.order,
        })
        .from(Entities)
        .innerJoin(Posts, eq(Entities.id, Posts.entityId))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      const nextEntity = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, entity.siteId),
            entity.parentEntityId ? eq(Entities.parentId, entity.parentEntityId) : isNull(Entities.parentId),
            gt(Entities.order, entity.order),
          ),
        )
        .orderBy(asc(Entities.order))
        .limit(1)
        .then(first);

      const post = await db
        .select({
          title: Posts.title,
          subtitle: Posts.subtitle,
          maxWidth: Posts.maxWidth,
          coverImageId: Posts.coverImageId,
          content: {
            body: PostContents.body,
            text: PostContents.text,
          },
          option: {
            allowComment: PostOptions.allowComment,
            allowReaction: PostOptions.allowReaction,
            protectContent: PostOptions.protectContent,
            password: PostOptions.password,
          },
        })
        .from(Posts)
        .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
        .innerJoin(PostOptions, eq(Posts.id, PostOptions.postId))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      const title = `(사본) ${post.title ?? '(제목 없음)'}`;

      const doc = makeYDoc({
        title,
        subtitle: post.subtitle,
        body: post.content.body,
        maxWidth: post.maxWidth,
      });

      const snapshot = Y.snapshot(doc);

      const newPost = await db.transaction(async (tx) => {
        const newEntity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: entity.siteId,
            parentId: entity.parentEntityId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.POST,
            order: generateEntityOrder({ lower: entity.order, upper: nextEntity?.order }),
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const newPost = await tx
          .insert(Posts)
          .values({
            entityId: newEntity.id,
            title,
            subtitle: post.subtitle,
            maxWidth: post.maxWidth,
            coverImageId: post.coverImageId,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(PostContents).values({
          postId: newPost.id,
          body: post.content.body,
          text: post.content.text,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });

        await tx.insert(PostSnapshots).values({
          userId: ctx.session.userId,
          postId: newPost.id,
          snapshot: Y.encodeSnapshotV2(snapshot),
        });

        await tx.insert(PostOptions).values({
          postId: newPost.id,
          visibility: PostVisibility.PRIVATE,
          ...post.option,
        });

        return newPost;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });

      return newPost;
    },
  }),

  deletePost: t.withAuth({ session: true }).fieldWithInput({
    type: Post,
    input: { postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }) },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ id: Entities.id, siteId: Entities.siteId })
        .from(Entities)
        .innerJoin(Posts, eq(Entities.id, Posts.entityId))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      await db
        .update(Entities)
        .set({
          state: EntityState.DELETED,
        })
        .where(eq(Entities.id, entity.id));

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });

      return input.postId;
    },
  }),

  updatePostOption: t.withAuth({ session: true }).fieldWithInput({
    type: PostOption,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      visibility: t.input.field({ type: PostVisibility }),
      password: t.input.string({ required: false }),
      contentRating: t.input.field({ type: PostContentRating }),
      allowComment: t.input.boolean(),
      allowReaction: t.input.boolean(),
      protectContent: t.input.boolean(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({ siteId: Entities.siteId })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: post.siteId,
      });

      return await db
        .update(PostOptions)
        .set({
          visibility: input.visibility,
          password: input.password,
          contentRating: input.contentRating,
          allowComment: input.allowComment,
          allowReaction: input.allowReaction,
          protectContent: input.protectContent,
        })
        .where(eq(PostOptions.postId, input.postId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  unlockPostView: t.fieldWithInput({
    type: PostView,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      password: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const postOption = await db
        .select({ password: PostOptions.password })
        .from(PostOptions)
        .where(eq(PostOptions.postId, input.postId))
        .then(firstOrThrow);

      if (postOption.password !== input.password) {
        throw new TypieError({ code: 'invalid_password' });
      }

      await redis.setex(`postview:unlock:${input.postId}:${ctx.deviceId}`, 60 * 60 * 24, 'true');

      return input.postId;
    },
  }),

  createPostReaction: t.fieldWithInput({
    type: PostReaction,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      emoji: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({
          state: Entities.state,
          allowReaction: PostOptions.allowReaction,
        })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .innerJoin(PostOptions, eq(Posts.id, PostOptions.postId))
        .where(eq(Posts.id, input.postId))
        .then(first);

      if (post?.state !== EntityState.ACTIVE) {
        throw new TypieError({ code: 'not_found' });
      }

      if (!post.allowReaction) {
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

/**
 * * Utils
 */

const generateSlug = () => faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' });
const generatePermalink = () => faker.string.alphanumeric({ length: 6, casing: 'mixed' });

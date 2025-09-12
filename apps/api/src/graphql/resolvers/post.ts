import { Node } from '@tiptap/pm/model';
import dayjs from 'dayjs';
import { and, asc, count, desc, eq, gt, gte, inArray, isNull, lt, sum } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { nanoid } from 'nanoid';
import { match } from 'ts-pattern';
import * as Y from 'yjs';
import { redis } from '@/cache';
import {
  Comments,
  db,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  PostAnchors,
  PostCharacterCountChanges,
  PostContents,
  PostReactions,
  Posts,
  PostSnapshotContributors,
  PostSnapshots,
  TableCode,
  UserPersonalIdentities,
  Users,
  validateDbId,
} from '@/db';
import {
  EntityAvailability,
  EntityState,
  EntityType,
  EntityVisibility,
  PostAvailableAction,
  PostContentRating,
  PostLayoutMode,
  PostSyncType,
  PostType,
  PostViewBodyUnavailableReason,
} from '@/enums';
import { env } from '@/env';
import { NotFoundError, TypieError } from '@/errors';
import * as slack from '@/external/slack';
import * as spellcheck from '@/external/spellcheck';
import { enqueueJob } from '@/mq';
import { schema, textSerializers } from '@/pm';
import { pubsub } from '@/pubsub';
import { generateEntityOrder, generatePermalink, generateSlug, getKoreanAge, makeText, makeYDoc } from '@/utils';
import { compressZstd, decompressZstd } from '@/utils/compression';
import { assertSitePermission } from '@/utils/permission';
import { assertPlanRule } from '@/utils/plan';
import { builder } from '../builder';
import {
  CharacterCountChange,
  Comment,
  Entity,
  EntityView,
  Image,
  IPost,
  isTypeOf,
  Post,
  PostReaction,
  PostSnapshot,
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

    contentRating: t.expose('contentRating', { type: PostContentRating }),
    allowComment: t.exposeBoolean('allowComment'),
    allowReaction: t.exposeBoolean('allowReaction'),
    protectContent: t.exposeBoolean('protectContent'),

    type: t.expose('type', { type: PostType }),

    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

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

    layoutMode: t.field({
      type: PostLayoutMode,
      resolve: async (self) => {
        const content = await db
          .select({ layoutMode: PostContents.layoutMode })
          .from(PostContents)
          .where(eq(PostContents.postId, self.id))
          .then(firstOrThrow);

        return content.layoutMode;
      },
    }),

    pageLayout: t.field({
      type: 'JSON',
      nullable: true,
      resolve: async (self) => {
        const content = await db
          .select({ pageLayout: PostContents.pageLayout })
          .from(PostContents)
          .where(eq(PostContents.postId, self.id))
          .then(firstOrThrow);

        return content.pageLayout;
      },
    }),

    availableActions: t.field({
      type: [PostAvailableAction],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Post.availableActions',
          load: async (ids) => {
            return await db
              .select({ postId: Posts.id, entityId: Entities.id, siteId: Entities.siteId })
              .from(Posts)
              .innerJoin(Entities, eq(Posts.entityId, Entities.id))
              .where(inArray(Posts.id, ids));
          },
          key: ({ postId }) => postId,
        });

        const post = await loader.load(self.id);

        return await Promise.allSettled([
          assertSitePermission({
            userId: ctx.session?.userId,
            siteId: post.siteId,
          }).then(() => PostAvailableAction.EDIT),
        ]).then((results) => results.filter((result) => result.status === 'fulfilled').flatMap((result) => result.value));
      },
    }),
  }),
});

Post.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    view: t.expose('id', { type: PostView }),

    password: t.exposeString('password', { nullable: true }),

    update: t.field({
      type: 'Binary',
      resolve: async (self) => {
        const content = await db
          .select({ update: PostContents.update })
          .from(PostContents)
          .where(eq(PostContents.postId, self.id))
          .then(firstOrThrow);

        return content.update;
      },
    }),

    snapshots: t.field({
      type: [PostSnapshot],
      resolve: async (self) => {
        return await db.select().from(PostSnapshots).where(eq(PostSnapshots.postId, self.id)).orderBy(asc(PostSnapshots.createdAt));
      },
    }),

    body: t.field({
      type: 'JSON',
      resolve: async (self) => {
        const content = await db
          .select({ body: PostContents.body })
          .from(PostContents)
          .where(eq(PostContents.postId, self.id))
          .then(firstOrThrow);

        return content.body;
      },
    }),

    storedMarks: t.field({
      type: 'JSON',
      resolve: async (self) => {
        const content = await db
          .select({ storedMarks: PostContents.storedMarks })
          .from(PostContents)
          .where(eq(PostContents.postId, self.id))
          .then(firstOrThrow);

        return content.storedMarks;
      },
    }),

    entity: t.expose('entityId', { type: Entity }),

    characterCount: t.int({
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Post.characterCount',
          load: async (ids) => {
            return await db
              .select({ postId: PostContents.postId, characterCount: PostContents.characterCount })
              .from(PostContents)
              .where(inArray(PostContents.postId, ids));
          },
          key: ({ postId }) => postId,
        });

        const content = await loader.load(self.id);
        return content.characterCount;
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

    reactionCount: t.int({
      resolve: async (self) => {
        const r = await db.select({ count: count() }).from(PostReactions).where(eq(PostReactions.postId, self.id)).then(firstOrThrow);
        return r.count;
      },
    }),
  }),
});

PostView.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    hasPassword: t.boolean({ resolve: (self) => !!self.password }),

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
        if (self.contentRating !== PostContentRating.ALL) {
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

          const minAge = match(self.contentRating)
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

        if (self.password !== null) {
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

    reactions: t.field({
      type: [PostReaction],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'PostView.reactions',
          many: true,
          load: async (ids) => {
            return await db.select().from(PostReactions).where(inArray(PostReactions.postId, ids)).orderBy(desc(PostReactions.createdAt));
          },
          key: ({ postId }) => postId,
        });

        return await loader.load(self.id);
      },
    }),

    comments: t.field({
      type: [Comment],
      resolve: async (self, _, ctx) => {
        if (!self.allowComment) {
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

        return await commentsLoader.load(self.id);
      },
    }),
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

PostSnapshot.implement({
  isTypeOf: isTypeOf(TableCode.POST_SNAPSHOTS),
  fields: (t) => ({
    id: t.exposeID('id'),
    snapshot: t.field({ type: 'Binary', resolve: (self) => decompressZstd(self.snapshot) }),
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
    resolve: async (_, args, ctx) => {
      const { post, entity } = await db
        .select({ post: Posts, entity: { siteId: Entities.siteId, availability: Entities.availability } })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Entities.slug, args.slug))
        .then(firstOrThrowWith(new NotFoundError()));

      if (entity.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: entity.siteId,
        }).catch(() => {
          throw new NotFoundError();
        });
      }

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

      const node = schema.nodes.doc.createChecked(
        null,
        schema.nodes.body.createChecked(
          { nodeId: nanoid(32), paragraphIndent: 1, blockGap: 1 },
          schema.nodes.paragraph.createChecked({ nodeId: nanoid(32), textAlign: 'left', lineHeight: 1.6, letterSpacing: 0 }),
        ),
      );

      const body = node.toJSON();
      const text = makeText(body);

      const doc = makeYDoc({ title, subtitle, body });
      const snapshot = Y.snapshot(doc);

      let depth = 0;
      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ id: Entities.id, depth: Entities.depth })
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

        depth = parentEntity.depth + 1;
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
            depth,
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

        const snapshotData = Y.encodeSnapshotV2(snapshot);
        const compressedSnapshot = await compressZstd(snapshotData);

        const postSnapshot = await tx
          .insert(PostSnapshots)
          .values({
            postId: post.id,
            snapshot: compressedSnapshot,
          })
          .returning({ id: PostSnapshots.id })
          .then(firstOrThrow);

        await tx.insert(PostSnapshotContributors).values({
          snapshotId: postSnapshot.id,
          userId: ctx.session.userId,
        });

        return post;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', input.siteId, null);

      await enqueueJob('post:index', post.id);

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
          depth: Entities.depth,
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
          allowComment: Posts.allowComment,
          allowReaction: Posts.allowReaction,
          protectContent: Posts.protectContent,
          password: Posts.password,
          content: {
            body: PostContents.body,
            text: PostContents.text,
            characterCount: PostContents.characterCount,
            blobSize: PostContents.blobSize,
            layoutMode: PostContents.layoutMode,
            pageLayout: PostContents.pageLayout,
            storedMarks: PostContents.storedMarks,
            note: PostContents.note,
          },
        })
        .from(Posts)
        .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      await assertPlanRule({ userId: ctx.session.userId, rule: 'maxTotalCharacterCount' });
      await assertPlanRule({ userId: ctx.session.userId, rule: 'maxTotalBlobSize' });

      const anchors = await db
        .select({ nodeId: PostAnchors.nodeId, name: PostAnchors.name })
        .from(PostAnchors)
        .where(eq(PostAnchors.postId, input.postId));

      const title = `(사본) ${post.title ?? '(제목 없음)'}`;

      const doc = makeYDoc({
        title,
        subtitle: post.subtitle,
        body: post.content.body,
        maxWidth: post.maxWidth,
        storedMarks: post.content.storedMarks,
        layoutMode: post.content.layoutMode,
        pageLayout: post.content.pageLayout,
        note: post.content.note,
        anchors: Object.fromEntries(anchors.map((anchor) => [anchor.nodeId, anchor.name])),
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
            depth: entity.depth,
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
            allowComment: post.allowComment,
            allowReaction: post.allowReaction,
            protectContent: post.protectContent,
            password: post.password,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(PostContents).values({
          postId: newPost.id,
          body: post.content.body,
          text: post.content.text,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
          characterCount: post.content.characterCount,
          blobSize: post.content.blobSize,
          layoutMode: post.content.layoutMode,
          pageLayout: post.content.pageLayout,
          note: post.content.note,
        });

        if (anchors.length > 0) {
          await tx.insert(PostAnchors).values(anchors.map((anchor) => ({ postId: newPost.id, nodeId: anchor.nodeId, name: anchor.name })));
        }

        const snapshotData = Y.encodeSnapshotV2(snapshot);
        const compressedSnapshot = await compressZstd(snapshotData);

        const postSnapshot = await tx
          .insert(PostSnapshots)
          .values({
            postId: newPost.id,
            snapshot: compressedSnapshot,
          })
          .returning({ id: PostSnapshots.id })
          .then(firstOrThrow);

        await tx.insert(PostSnapshotContributors).values({
          snapshotId: postSnapshot.id,
          userId: ctx.session.userId,
        });

        return newPost;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', entity.siteId, null);

      await enqueueJob('post:index', newPost.id);

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
          deletedAt: dayjs(),
        })
        .where(eq(Entities.id, entity.id));

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.id });
      pubsub.publish('site:usage:update', entity.siteId, null);

      await enqueueJob('post:index', input.postId);

      return input.postId;
    },
  }),

  updatePostOption: t.withAuth({ session: true }).fieldWithInput({
    type: Post,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      availability: t.input.field({ type: EntityAvailability, required: false }),
      visibility: t.input.field({ type: EntityVisibility }),
      password: t.input.string({ required: false }),
      contentRating: t.input.field({ type: PostContentRating }),
      allowComment: t.input.boolean({ required: false }),
      allowReaction: t.input.boolean(),
      protectContent: t.input.boolean(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({ siteId: Entities.siteId, entityId: Entities.id })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: post.siteId,
      });

      return await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({
            availability: input.availability ?? undefined,
            visibility: input.visibility,
          })
          .where(eq(Entities.id, post.entityId));

        return await tx
          .update(Posts)
          .set({
            password: input.password || null,
            contentRating: input.contentRating,
            allowComment: input.allowComment ?? true,
            allowReaction: input.allowReaction,
            protectContent: input.protectContent,
          })
          .where(eq(Posts.id, input.postId))
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  updatePostsOption: t.withAuth({ session: true }).fieldWithInput({
    type: [Post],
    input: {
      postIds: t.input.idList({ validate: { items: validateDbId(TableCode.POSTS) } }),
      availability: t.input.field({ type: EntityAvailability, required: false }),
      visibility: t.input.field({ type: EntityVisibility, required: false }),
      password: t.input.string({ required: false }),
      contentRating: t.input.field({ type: PostContentRating, required: false }),
      allowReaction: t.input.boolean({ required: false }),
      protectContent: t.input.boolean({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const posts = await db
        .select({
          id: Posts.id,
          siteId: Entities.siteId,
          entityId: Entities.id,
        })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(and(eq(Entities.state, EntityState.ACTIVE), inArray(Posts.id, input.postIds)));

      if (posts.length === 0) {
        throw new TypieError({ code: 'invalid_argument' });
      }

      const siteId = posts[0].siteId;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      if (posts.some((post) => post.siteId !== siteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      const updatedPostIds = await db.transaction(async (tx) => {
        if (input.availability || input.visibility) {
          // availability, visibility는 not null이므로 null이여도 undefined로 취급
          await tx
            .update(Entities)
            .set({
              availability: input.availability ?? undefined,
              visibility: input.visibility ?? undefined,
            })
            .where(
              inArray(
                Entities.id,
                posts.map((post) => post.entityId),
              ),
            );
        }

        if (
          input.contentRating ||
          typeof input.allowReaction === 'boolean' ||
          typeof input.protectContent === 'boolean' ||
          input.password !== undefined
        ) {
          // 상동, password는 nullable이므로 null과 undefined 구분
          await tx
            .update(Posts)
            .set({
              contentRating: input.contentRating ?? undefined,
              allowReaction: input.allowReaction ?? undefined,
              protectContent: input.protectContent ?? undefined,
              password: input.password,
            })
            .where(
              inArray(
                Posts.id,
                posts.map((post) => post.id),
              ),
            );
        }

        return posts.map((post) => post.id);
      });

      pubsub.publish('site:update', siteId, { scope: 'site' });
      for (const post of posts) {
        pubsub.publish('site:update', siteId, { scope: 'entity', entityId: post.entityId });
      }

      return updatedPostIds;
    },
  }),

  updatePostType: t.withAuth({ session: true }).fieldWithInput({
    type: Post,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      type: t.input.field({ type: PostType }),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({ siteId: Entities.siteId, entityId: Entities.id })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: post.siteId,
      });

      const updatedPost = await db
        .update(Posts)
        .set({
          type: input.type,
        })
        .where(eq(Posts.id, input.postId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('site:update', post.siteId, { scope: 'site' });
      pubsub.publish('site:update', post.siteId, { scope: 'entity', entityId: post.entityId });

      return updatedPost;
    },
  }),

  unlockPostView: t.fieldWithInput({
    type: PostView,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      password: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db.select({ password: Posts.password }).from(Posts).where(eq(Posts.id, input.postId)).then(firstOrThrow);

      if (post.password !== input.password) {
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
          allowReaction: Posts.allowReaction,
        })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
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

  syncPost: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      clientId: t.input.string(),
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      type: t.input.field({ type: PostSyncType }),
      data: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({ siteId: Entities.siteId, availability: Entities.availability })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      if (post.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: post.siteId,
        });
      }

      if (input.type === PostSyncType.UPDATE) {
        pubsub.publish('post:sync', input.postId, {
          target: `!${input.clientId}`,
          type: PostSyncType.UPDATE,
          data: input.data,
        });

        await redis.sadd(
          `post:sync:updates:${input.postId}`,
          JSON.stringify({
            userId: ctx.session.userId,
            data: input.data,
          }),
        );

        await enqueueJob('post:sync:collect', input.postId, {
          deduplication: {
            id: input.postId,
          },
        });
      } else if (input.type === PostSyncType.VECTOR) {
        const state = await getPostDocument(input.postId);
        const update = Y.diffUpdateV2(state.update, Uint8Array.fromBase64(input.data));

        pubsub.publish('post:sync', input.postId, {
          target: input.clientId,
          type: PostSyncType.UPDATE,
          data: update.toBase64(),
        });

        pubsub.publish('post:sync', input.postId, {
          target: input.clientId,
          type: PostSyncType.VECTOR,
          data: state.vector.toBase64(),
        });
      } else if (input.type === PostSyncType.AWARENESS) {
        pubsub.publish('post:sync', input.postId, {
          target: `!${input.clientId}`,
          type: PostSyncType.AWARENESS,
          data: input.data,
        });
      }

      return true;
    },
  }),

  reportPost: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      reason: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({
          id: Posts.id,
          title: Posts.title,
          permalink: Entities.permalink,
        })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      const user = await db
        .select({ id: Users.id, name: Users.name, email: Users.email })
        .from(Users)
        .where(eq(Users.id, ctx.session.userId))
        .then(firstOrThrow);

      await slack.sendMessage({
        channel: '#cs',
        username: '타이피 신고 알림',
        iconEmoji: ':rotating_light:',
        message: `${post.title} (${post.id}) 포스트 신고
        신고자: ${user.name}(${user.id}, ${user.email})
        이유: ${input.reason}
        ${env.USERSITE_URL.replace('*.', '')}/${post.permalink}`,
      });

      return true;
    },
  }),

  checkSpelling: t.withAuth({ session: true }).fieldWithInput({
    type: [
      builder.simpleObject('SpellingError', {
        fields: (t) => ({
          from: t.int(),
          to: t.int(),
          context: t.string(),
          corrections: t.stringList(),
          explanation: t.string(),
        }),
      }),
    ],
    input: { body: t.input.field({ type: 'JSON' }) },
    resolve: async (_, { input }) => {
      const node = Node.fromJSON(schema, input.body);

      let text = '';
      let textOffset = 0;

      const textNodeMappings: { textStart: number; textEnd: number; pmStart: number }[] = [];

      node.nodesBetween(0, node.content.size, (childNode, pos, parent, index) => {
        const textSerializer = textSerializers[childNode.type.name];
        if (textSerializer) {
          if (parent) {
            const range = { from: 0, to: node.content.size };
            const serialized = textSerializer({ node: childNode, pos, parent, index, range });
            text += serialized;
            textOffset += serialized.length;
          }

          return false;
        }

        if (childNode.isBlock && pos > 0) {
          text += '\n';
          textOffset += 1;
        }

        if (childNode.isText) {
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          const content = childNode.text!;

          textNodeMappings.push({
            textStart: textOffset,
            textEnd: textOffset + content.length,
            pmStart: pos,
          });

          text += content;
          textOffset += content.length;
        }
      });

      const errors = await spellcheck.check(text);

      const mapRange = (textStart: number, textEnd: number) => {
        const startMapping = textNodeMappings.find((m) => textStart >= m.textStart && textStart < m.textEnd);
        const endMapping = textNodeMappings.find((m) => textEnd > m.textStart && textEnd <= m.textEnd);

        if (!startMapping || !endMapping || startMapping !== endMapping) {
          return null;
        }

        const from = startMapping.pmStart + (textStart - startMapping.textStart);
        const to = startMapping.pmStart + (textEnd - startMapping.textStart);

        return { from, to };
      };

      return errors
        .map((error) => {
          const range = mapRange(error.start, error.end);

          return {
            from: range?.from,
            to: range?.to,
            context: error.context,
            corrections: error.corrections,
            explanation: error.explanation,
          };
        })
        .filter(
          (error): error is { from: number; to: number; context: string; corrections: string[]; explanation: string } =>
            error.from !== undefined && error.to !== undefined,
        );
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  postSyncStream: t.withAuth({ session: true }).field({
    type: t.builder.simpleObject('PostSyncStreamPayload', {
      fields: (t) => ({
        postId: t.id(),
        type: t.field({ type: PostSyncType }),
        data: t.string(),
      }),
    }),
    args: {
      clientId: t.arg.string(),
      postId: t.arg.id({ validate: validateDbId(TableCode.POSTS) }),
    },
    subscribe: async (_, args, ctx) => {
      const post = await db
        .select({ siteId: Entities.siteId, availability: Entities.availability })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, args.postId))
        .then(firstOrThrow);

      if (post.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: post.siteId,
        });
      }

      pubsub.publish('post:sync', args.postId, {
        target: `!${args.clientId}`,
        type: PostSyncType.PRESENCE,
        data: '',
      });

      const repeater = Repeater.merge([
        pubsub.subscribe('post:sync', args.postId),
        new Repeater<{ target: string; type: PostSyncType; data: string }>(async (push, stop) => {
          const heartbeat = () => {
            push({
              target: args.clientId,
              type: PostSyncType.HEARTBEAT,
              data: dayjs().toISOString(),
            });
          };

          heartbeat();
          const interval = setInterval(heartbeat, 1000);

          await stop;

          clearInterval(interval);
        }),
      ]);

      return pipe(
        repeater,
        filter(({ target }) => {
          if (target === '*') {
            return true;
          } else if (target.startsWith('!')) {
            return target.slice(1) !== args.clientId;
          } else {
            return target === args.clientId;
          }
        }),
      );
    },
    resolve: async (payload, args) => {
      return {
        postId: args.postId,
        type: payload.type,
        data: payload.data,
      };
    },
  }),
}));

/**
 * * Utils
 */

const getPostDocument = async (postId: string) => {
  const { update, vector } = await db
    .select({ update: PostContents.update, vector: PostContents.vector })
    .from(PostContents)
    .where(eq(PostContents.postId, postId))
    .then(firstOrThrow);

  const updates = await redis.smembers(`post:sync:updates:${postId}`);
  if (updates.length === 0) {
    return {
      update,
      vector,
    };
  }

  const pendingUpdates = updates.map((update) => {
    const { data } = JSON.parse(update);
    return Uint8Array.fromBase64(data);
  });

  const updatedUpdate = Y.mergeUpdatesV2([update, ...pendingUpdates]);
  const updatedVector = Y.encodeStateVectorFromUpdateV2(updatedUpdate);

  return {
    update: updatedUpdate,
    vector: updatedVector,
  };
};

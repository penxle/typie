import dayjs from 'dayjs';
import { TableCode } from '@/db';
import { PostAvailableAction, PostContentRating, PostLayoutMode, PostType, PostViewBodyUnavailableReason } from '@/enums';
import { NotFoundError } from '@/errors';
import { builder } from '../builder';
import {
  CharacterCountChange,
  Document,
  DocumentView,
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

const PostSnapshotMeta = builder.simpleObject('PostSnapshotMeta', {
  fields: (t) => ({
    id: t.id(),
    createdAt: t.field({ type: 'DateTime' }),
  }),
});

IPost.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.string({ resolve: () => '' }),
    subtitle: t.string({ nullable: true, resolve: () => null }),
    maxWidth: t.int({ resolve: () => 0 }),
    coverImage: t.field({ type: Image, nullable: true, resolve: () => null }),
    thumbnail: t.field({ type: Image, nullable: true, resolve: () => null }),

    contentRating: t.field({ type: PostContentRating, resolve: () => PostContentRating.ALL }),
    allowComment: t.boolean({ resolve: () => false }),
    allowReaction: t.boolean({ resolve: () => false }),
    protectContent: t.boolean({ resolve: () => false }),

    type: t.field({ type: PostType, resolve: () => PostType.NORMAL }),

    createdAt: t.field({ type: 'DateTime', resolve: () => dayjs(0) }),
    updatedAt: t.field({ type: 'DateTime', resolve: () => dayjs(0) }),

    excerpt: t.string({ resolve: () => '' }),

    layoutMode: t.field({ type: PostLayoutMode, resolve: () => PostLayoutMode.SCROLL }),

    pageLayout: t.field({ type: 'JSON', nullable: true, resolve: () => null }),

    availableActions: t.field({ type: [PostAvailableAction], resolve: () => [] }),
  }),
});

Post.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    view: t.expose('id', { type: PostView }),

    password: t.string({ nullable: true, resolve: () => null }),

    document: t.expose('documentId', { type: Document, nullable: true }),

    update: t.field({ type: 'Binary', resolve: () => new Uint8Array(0) }),

    snapshots: t.field({
      type: [PostSnapshot],
      args: {
        first: t.arg.int({ defaultValue: 20 }),
        before: t.arg({ type: 'DateTime', required: false }),
      },
      resolve: () => [],
    }),

    snapshotMetas: t.field({ type: [PostSnapshotMeta], resolve: () => [] }),

    body: t.field({ type: 'JSON', resolve: () => ({}) }),

    storedMarks: t.field({ type: 'JSON', resolve: () => [] }),

    entity: t.expose('entityId', { type: Entity }),

    characterCount: t.int({ resolve: () => 0 }),

    characterCountChange: t.withAuth({ session: true }).field({
      type: CharacterCountChange,
      resolve: () => ({ date: dayjs(0), additions: 0, deletions: 0 }),
    }),

    reactionCount: t.int({ resolve: () => 0 }),
  }),
});

PostView.implement({
  isTypeOf: isTypeOf(TableCode.POSTS),
  interfaces: [IPost],
  fields: (t) => ({
    hasPassword: t.boolean({ resolve: () => false }),

    document: t.expose('documentId', { type: DocumentView, nullable: true }),

    excerpt: t.string({ resolve: () => '' }),

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
      resolve: () => ({
        __typename: 'PostViewBodyAvailable' as const,
        content: {},
      }),
    }),

    entity: t.expose('entityId', { type: EntityView }),

    reactions: t.field({ type: [PostReaction], resolve: () => [] }),
  }),
});

PostReaction.implement({
  isTypeOf: isTypeOf(TableCode.POST_REACTIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    emoji: t.string({ resolve: () => '' }),

    post: t.expose('postId', { type: PostView }),
  }),
});

PostSnapshot.implement({
  isTypeOf: isTypeOf(TableCode.POST_SNAPSHOTS),
  fields: (t) => ({
    id: t.exposeID('id'),
    snapshot: t.field({ type: 'Binary', resolve: () => new Uint8Array(0) }),
    createdAt: t.field({ type: 'DateTime', resolve: () => dayjs(0) }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  post: t.withAuth({ session: true }).field({
    type: Post,
    args: { slug: t.arg.string() },
    resolve: async () => {
      throw new NotFoundError();
    },
  }),
}));

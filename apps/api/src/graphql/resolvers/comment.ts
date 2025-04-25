import { and, eq } from 'drizzle-orm';
import { Comments, db, Entities, firstOrThrow, PostOptions, Posts, SiteProfiles, TableCode, validateDbId } from '@/db';
import { CommentState, EntityState, NotificationCategory } from '@/enums';
import { TypieError } from '@/errors';
import { enqueueJob } from '@/mq';
import { builder } from '../builder';
import { Comment, isTypeOf, SiteProfile } from '../objects';

Comment.implement({
  isTypeOf: isTypeOf(TableCode.COMMENTS),
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: CommentState }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),

    profile: t.field({
      type: SiteProfile,
      nullable: true,
      resolve: (self) => (self.state === CommentState.ACTIVE ? self.profileId : null),
    }),

    content: t.string({
      nullable: true,
      resolve: (self) => (self.state === CommentState.ACTIVE ? self.content : null),
    }),
  }),
});

builder.mutationFields((t) => ({
  createComment: t.withAuth({ session: true }).fieldWithInput({
    type: Comment,
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      content: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const post = await db
        .select({ id: Posts.id, userId: Entities.userId, siteId: Entities.siteId })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .innerJoin(PostOptions, eq(Posts.id, PostOptions.postId))
        .where(and(eq(Posts.id, input.postId), eq(Entities.state, EntityState.ACTIVE), eq(PostOptions.allowComment, true)))
        .then(firstOrThrow);

      const profile = await db
        .select({ id: SiteProfiles.id })
        .from(SiteProfiles)
        .where(and(eq(SiteProfiles.siteId, post.siteId), eq(SiteProfiles.userId, ctx.session.userId)))
        .then(firstOrThrow);

      const comment = await db
        .insert(Comments)
        .values({
          postId: post.id,
          profileId: profile.id,
          userId: ctx.session.userId,
          content: input.content,
        })
        .returning()
        .then(firstOrThrow);

      await enqueueJob('notification:create', {
        userId: post.userId,
        data: {
          category: NotificationCategory.COMMENT,
          commentId: comment.id,
        },
      });

      return comment;
    },
  }),

  deleteComment: t.withAuth({ session: true }).fieldWithInput({
    type: Comment,
    input: { commentId: t.input.id({ validate: validateDbId(TableCode.COMMENTS) }) },
    resolve: async (_, { input }, ctx) => {
      const { comment, entity } = await db
        .select({ comment: Comments, entity: { userId: Entities.userId } })
        .from(Comments)
        .innerJoin(Posts, eq(Comments.postId, Posts.id))
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Comments.id, input.commentId))
        .then(firstOrThrow);

      if (comment.userId !== ctx.session.userId && entity.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'forbidden' });
      }

      return await db
        .update(Comments)
        .set({ state: CommentState.DELETED })
        .where(eq(Comments.id, comment.id))
        .returning()
        .then(firstOrThrow);
    },
  }),
}));

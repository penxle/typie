import { and, eq, inArray } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { Comments, db, firstOrThrow, Notifications, TableCode, validateDbId } from '@/db';
import { NotificationCategory, NotificationState } from '@/enums';
import { builder } from '../builder';
import { Comment, Notification, Post } from '../objects';
import type { NotificationAnnouncementData, NotificationCommentData } from '@/db/schemas/json';

Notification.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: NotificationState }),
    data: t.expose('data', { type: NotificationData }),
  }),
});

const NotificationAnnouncementData = builder.objectRef<NotificationAnnouncementData>('NotificationAnnouncementData').implement({
  fields: (t) => ({
    message: t.exposeString('message'),
    link: t.exposeString('link', { nullable: true }),
  }),
});

const NotificationCommentData = builder.objectRef<NotificationCommentData>('NotificationCommentData').implement({
  fields: (t) => ({
    comment: t.expose('commentId', { type: Comment }),

    post: t.field({
      type: Post,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'NotificationCommentData.post',
          load: (ids) => {
            return db.select({ id: Comments.id, postId: Comments.postId }).from(Comments).where(inArray(Comments.id, ids));
          },
          key: ({ id }) => id,
        });

        const comment = await loader.load(self.commentId);
        return comment.postId;
      },
    }),
  }),
});

const NotificationData = builder.unionType('NotificationData', {
  types: [NotificationCommentData, NotificationAnnouncementData],
  resolveType: (self) =>
    match(self.category)
      .with(NotificationCategory.COMMENT, () => NotificationCommentData)
      .with(NotificationCategory.ANNOUNCEMENT, () => NotificationAnnouncementData)
      .exhaustive(),
});

builder.mutationFields((t) => ({
  markNotificationAsRead: t.withAuth({ session: true }).fieldWithInput({
    type: Notification,
    input: {
      notificationId: t.input.id({ validate: validateDbId(TableCode.NOTIFICATIONS) }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .update(Notifications)
        .set({ state: NotificationState.READ })
        .where(and(eq(Notifications.id, input.notificationId), eq(Notifications.userId, ctx.session.userId)))
        .returning()
        .then(firstOrThrow);
    },
  }),

  markAllNotificationsAsRead: t.withAuth({ session: true }).field({
    type: [Notification],
    resolve: async (_, __, ctx) => {
      return await db
        .update(Notifications)
        .set({ state: NotificationState.READ })
        .where(and(eq(Notifications.userId, ctx.session.userId), eq(Notifications.state, NotificationState.UNREAD)))
        .returning();
    },
  }),
}));

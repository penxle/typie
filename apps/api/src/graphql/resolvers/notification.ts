import { and, eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, firstOrThrow, Notifications } from '@/db';
import { NotificationCategory, NotificationState } from '@/enums';
import { builder } from '../builder';
import { Comment, Notification } from '../objects';
import type { AnnouncementNotificationData, CommentNotificationData } from '@/db/schemas/json';

Notification.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: NotificationState }),
    category: t.expose('category', { type: NotificationCategory }),
    data: t.expose('data', { type: NotificationData }),
  }),
});

const CommentNotificationData = builder.objectRef<CommentNotificationData>('CommentNotificationData').implement({
  fields: (t) => ({
    comment: t.field({
      type: Comment,
      resolve: (data) => data.commentId,
    }),
  }),
});

const AnnouncementNotificationData = builder.objectRef<AnnouncementNotificationData>('AnnouncementNotificationData').implement({
  fields: (t) => ({
    message: t.exposeString('message'),
    link: t.exposeString('link', { nullable: true }),
  }),
});

const NotificationData = builder.unionType('NotificationData', {
  types: [CommentNotificationData, AnnouncementNotificationData],
  resolveType: (data) =>
    match(data.category)
      .with(NotificationCategory.COMMENT, () => CommentNotificationData)
      .with(NotificationCategory.ANNOUNCEMENT, () => AnnouncementNotificationData)
      .exhaustive(),
});

builder.mutationFields((t) => ({
  markNotificationAsRead: t.withAuth({ session: true }).fieldWithInput({
    type: Notification,
    input: {
      id: t.input.id(),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .update(Notifications)
        .set({ state: NotificationState.READ })
        .where(and(eq(Notifications.id, input.id), eq(Notifications.userId, ctx.session.userId)))
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

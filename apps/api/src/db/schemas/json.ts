import { NotificationCategory } from '@/enums';

export type PlanRules = {
  writePost: boolean;
};

export const defaultPlanRules: PlanRules = {
  writePost: false,
};

export type CommentNotificationData = {
  category: typeof NotificationCategory.COMMENT;
  commentId: string;
};

export type AnnouncementNotificationData = {
  category: typeof NotificationCategory.ANNOUNCEMENT;
  message: string;
  link?: string;
};

export type NotificationData = CommentNotificationData | AnnouncementNotificationData;

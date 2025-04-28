import { NotificationCategory } from '@/enums';

export type PlanRules = {
  maxPostCount: number;
};

export const defaultPlanRules: PlanRules = {
  maxPostCount: -1,
};

export type NotificationAnnouncementData = {
  category: typeof NotificationCategory.ANNOUNCEMENT;
  message: string;
  link?: string;
};

export type NotificationCommentData = {
  category: typeof NotificationCategory.COMMENT;
  commentId: string;
};

export type NotificationData = NotificationAnnouncementData | NotificationCommentData;

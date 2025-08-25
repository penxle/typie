import { NotificationCategory } from '@/enums';

export type PlanRules = {
  maxTotalCharacterCount: number;
  maxTotalBlobSize: number;
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

export type CanvasShape = {
  type: string;
  attrs: Record<string, unknown>;
};

export type PageLayout = {
  width: number;
  height: number;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
};

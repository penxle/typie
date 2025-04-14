export type CommentState = keyof typeof CommentState;
export const CommentState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type EntityState = keyof typeof EntityState;
export const EntityState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type EntityType = keyof typeof EntityType;
export const EntityType = {
  FOLDER: 'FOLDER',
  POST: 'POST',
} as const;

export type JobState = keyof typeof JobState;
export const JobState = {
  PENDING: 'PENDING',
  RUNNING: 'RUNNING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
} as const;

export type NotificationCategory = keyof typeof NotificationCategory;
export const NotificationCategory = {
  ANNOUNCEMENT: 'ANNOUNCEMENT',
  COMMENT: 'COMMENT',
} as const;

export type NotificationState = keyof typeof NotificationState;
export const NotificationState = {
  UNREAD: 'UNREAD',
  READ: 'READ',
} as const;

export type PaymentInvoiceState = keyof typeof PaymentInvoiceState;
export const PaymentInvoiceState = {
  UPCOMING: 'UPCOMING',
  PAID: 'PAID',
  UNPAID: 'UNPAID',
  CANCELED: 'CANCELED',
} as const;

export type PaymentMethodState = keyof typeof PaymentMethodState;
export const PaymentMethodState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

export type PaymentRecordState = keyof typeof PaymentRecordState;
export const PaymentRecordState = {
  SUCCEEDED: 'SUCCEEDED',
  FAILED: 'FAILED',
} as const;

export type PlanAvailability = keyof typeof PlanAvailability;
export const PlanAvailability = {
  PUBLIC: 'PUBLIC',
  PRIVATE: 'PRIVATE',
} as const;

export type PostVisibility = keyof typeof PostVisibility;
export const PostVisibility = {
  UNLISTED: 'UNLISTED',
  PRIVATE: 'PRIVATE',
} as const;

export type PreorderPaymentState = keyof typeof PreorderPaymentState;
export const PreorderPaymentState = {
  PENDING: 'PENDING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
} as const;

export type SearchHitType = keyof typeof SearchHitType;
export const SearchHitType = {
  POST: 'POST',
} as const;

export type SingleSignOnProvider = keyof typeof SingleSignOnProvider;
export const SingleSignOnProvider = {
  GOOGLE: 'GOOGLE',
  KAKAO: 'KAKAO',
  NAVER: 'NAVER',
} as const;

export type SiteState = keyof typeof SiteState;
export const SiteState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type UserPlanBillingCycle = keyof typeof UserPlanBillingCycle;
export const UserPlanBillingCycle = {
  MONTHLY: 'MONTHLY',
  YEARLY: 'YEARLY',
} as const;

export type UserPlanState = keyof typeof UserPlanState;
export const UserPlanState = {
  ACTIVE: 'ACTIVE',
  CANCELED: 'CANCELED',
} as const;

export type UserState = keyof typeof UserState;
export const UserState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

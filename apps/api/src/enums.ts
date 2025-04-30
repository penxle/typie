export type CommentState = keyof typeof CommentState;
export const CommentState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type CreditCodeState = keyof typeof CreditCodeState;
export const CreditCodeState = {
  AVAILABLE: 'AVAILABLE',
  USED: 'USED',
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

export type EntityVisibility = keyof typeof EntityVisibility;
export const EntityVisibility = {
  UNLISTED: 'UNLISTED',
  PRIVATE: 'PRIVATE',
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

export type PaymentBillingKeyState = keyof typeof PaymentBillingKeyState;
export const PaymentBillingKeyState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

export type PaymentInvoiceState = keyof typeof PaymentInvoiceState;
export const PaymentInvoiceState = {
  UPCOMING: 'UPCOMING',
  PAID: 'PAID',
  UNPAID: 'UNPAID',
  CANCELED: 'CANCELED',
} as const;

export type PaymentMethodType = keyof typeof PaymentMethodType;
export const PaymentMethodType = {
  BILLING_KEY: 'BILLING_KEY',
  CREDIT: 'CREDIT',
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

export type PostContentRating = keyof typeof PostContentRating;
export const PostContentRating = {
  ALL: 'ALL',
  R15: 'R15',
  R19: 'R19',
} as const;

export type PostSyncType = keyof typeof PostSyncType;
export const PostSyncType = {
  HEARTBEAT: 'HEARTBEAT',
  UPDATE: 'UPDATE',
  VECTOR: 'VECTOR',
  AWARENESS: 'AWARENESS',
  PRESENCE: 'PRESENCE',
} as const;

export type PostViewBodyUnavailableReason = keyof typeof PostViewBodyUnavailableReason;
export const PostViewBodyUnavailableReason = {
  REQUIRE_PASSWORD: 'REQUIRE_PASSWORD',
  REQUIRE_IDENTITY_VERIFICATION: 'REQUIRE_IDENTITY_VERIFICATION',
  REQUIRE_MINIMUM_AGE: 'REQUIRE_MINIMUM_AGE',
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

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

export type EntityAvailability = keyof typeof EntityAvailability;
export const EntityAvailability = {
  PRIVATE: 'PRIVATE',
  UNLISTED: 'UNLISTED',
} as const;

export type EntityState = keyof typeof EntityState;
export const EntityState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type EntityType = keyof typeof EntityType;
export const EntityType = {
  CANVAS: 'CANVAS',
  FOLDER: 'FOLDER',
  POST: 'POST',
} as const;

export type EntityVisibility = keyof typeof EntityVisibility;
export const EntityVisibility = {
  UNLISTED: 'UNLISTED',
  PRIVATE: 'PRIVATE',
} as const;

export type FontState = keyof typeof FontState;
export const FontState = {
  ACTIVE: 'ACTIVE',
  ARCHIVED: 'ARCHIVED',
} as const;

export type InAppPurchaseStore = keyof typeof InAppPurchaseStore;
export const InAppPurchaseStore = {
  APP_STORE: 'APP_STORE',
  GOOGLE_PLAY: 'GOOGLE_PLAY',
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
  OVERDUE: 'OVERDUE',
  CANCELED: 'CANCELED',
} as const;

export type PaymentOutcome = keyof typeof PaymentOutcome;
export const PaymentOutcome = {
  SUCCESS: 'SUCCESS',
  FAILURE: 'FAILURE',
} as const;

export type PlanAvailability = keyof typeof PlanAvailability;
export const PlanAvailability = {
  BILLING_KEY: 'BILLING_KEY',
  IN_APP_PURCHASE: 'IN_APP_PURCHASE',
  MANUAL: 'MANUAL',
} as const;

export type PlanInterval = keyof typeof PlanInterval;
export const PlanInterval = {
  MONTHLY: 'MONTHLY',
  YEARLY: 'YEARLY',
  LIFETIME: 'LIFETIME',
} as const;

export const PostAvailableAction = {
  EDIT: 'EDIT',
} as const;

export type PostContentRating = keyof typeof PostContentRating;
export const PostContentRating = {
  ALL: 'ALL',
  R15: 'R15',
  R19: 'R19',
} as const;

export type PostType = keyof typeof PostType;
export const PostType = {
  NORMAL: 'NORMAL',
  TEMPLATE: 'TEMPLATE',
} as const;

export type PostSyncType = keyof typeof PostSyncType;
export const PostSyncType = {
  HEARTBEAT: 'HEARTBEAT',
  UPDATE: 'UPDATE',
  VECTOR: 'VECTOR',
  AWARENESS: 'AWARENESS',
  PRESENCE: 'PRESENCE',
} as const;

export type CanvasSyncType = keyof typeof CanvasSyncType;
export const CanvasSyncType = {
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
  APPLE: 'APPLE',
  GOOGLE: 'GOOGLE',
  KAKAO: 'KAKAO',
  NAVER: 'NAVER',
} as const;

export type SiteState = keyof typeof SiteState;
export const SiteState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type SubscriptionState = keyof typeof SubscriptionState;
export const SubscriptionState = {
  ACTIVE: 'ACTIVE',
  WILL_ACTIVATE: 'WILL_ACTIVATE',
  WILL_EXPIRE: 'WILL_EXPIRE',
  IN_GRACE_PERIOD: 'IN_GRACE_PERIOD',
  EXPIRED: 'EXPIRED',
} as const;

export type UserRole = keyof typeof UserRole;
export const UserRole = {
  ADMIN: 'ADMIN',
  USER: 'USER',
} as const;

export type UserState = keyof typeof UserState;
export const UserState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

export type CouponState = keyof typeof CouponState;
export const CouponState = {
  ACTIVE: 'ACTIVE',
  DISABLED: 'DISABLED',
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
  PURGED: 'PURGED',
} as const;

export type EntityType = keyof typeof EntityType;
export const EntityType = {
  DOCUMENT: 'DOCUMENT',
  FOLDER: 'FOLDER',
  POST: 'POST',
} as const;

export type EntityVisibility = keyof typeof EntityVisibility;
export const EntityVisibility = {
  PUBLIC: 'PUBLIC',
  UNLISTED: 'UNLISTED',
  PRIVATE: 'PRIVATE',
} as const;

export type FontFamilySource = keyof typeof FontFamilySource;
export const FontFamilySource = {
  DEFAULT: 'DEFAULT',
  USER: 'USER',
} as const;

export type FontState = keyof typeof FontState;
export const FontState = {
  ACTIVE: 'ACTIVE',
  ARCHIVED: 'ARCHIVED',
} as const;

export type FontFamilyState = keyof typeof FontFamilyState;
export const FontFamilyState = {
  ACTIVE: 'ACTIVE',
  ARCHIVED: 'ARCHIVED',
} as const;

export type InAppPurchaseStore = keyof typeof InAppPurchaseStore;
export const InAppPurchaseStore = {
  APP_STORE: 'APP_STORE',
  GOOGLE_PLAY: 'GOOGLE_PLAY',
} as const;

export type IssueState = keyof typeof IssueState;
export const IssueState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type IssuePriority = keyof typeof IssuePriority;
export const IssuePriority = {
  NONE: 'NONE',
  LOW: 'LOW',
  MEDIUM: 'MEDIUM',
  HIGH: 'HIGH',
  URGENT: 'URGENT',
} as const;

export type IssueStatus = keyof typeof IssueStatus;
export const IssueStatus = {
  OPEN: 'OPEN',
  IN_PROGRESS: 'IN_PROGRESS',
  RESOLVED: 'RESOLVED',
  CLOSED: 'CLOSED',
} as const;

export type NoteState = keyof typeof NoteState;
export const NoteState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
  DELETED_CASCADED: 'DELETED_CASCADED',
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
  TRIAL: 'TRIAL',
  MANUAL: 'MANUAL',
} as const;

export type PlanInterval = keyof typeof PlanInterval;
export const PlanInterval = {
  MONTHLY: 'MONTHLY',
  YEARLY: 'YEARLY',
  TRIAL: 'TRIAL',
  LIFETIME: 'LIFETIME',
} as const;

export const DocumentAvailableAction = {
  EDIT: 'EDIT',
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

export type PostLayoutMode = keyof typeof PostLayoutMode;
export const PostLayoutMode = {
  SCROLL: 'SCROLL',
  PAGE: 'PAGE',
} as const;

export type PostType = keyof typeof PostType;
export const PostType = {
  NORMAL: 'NORMAL',
  TEMPLATE: 'TEMPLATE',
} as const;

export type DocumentExportFormat = keyof typeof DocumentExportFormat;
export const DocumentExportFormat = {
  DOCX: 'DOCX',
  EPUB: 'EPUB',
  HWP: 'HWP',
  PDF: 'PDF',
} as const;

export type DocumentSyncType = keyof typeof DocumentSyncType;
export const DocumentSyncType = {
  HEARTBEAT: 'HEARTBEAT',
  UPDATE: 'UPDATE',
  VECTOR: 'VECTOR',
  AWARENESS: 'AWARENESS',
  PRESENCE: 'PRESENCE',
  RESET: 'RESET',
} as const;

export type DocumentType = keyof typeof DocumentType;
export const DocumentType = {
  NORMAL: 'NORMAL',
  TEMPLATE: 'TEMPLATE',
} as const;

export type DocumentContentRating = keyof typeof DocumentContentRating;
export const DocumentContentRating = {
  ALL: 'ALL',
  R15: 'R15',
  R19: 'R19',
} as const;

export type DocumentViewBodyUnavailableReason = keyof typeof DocumentViewBodyUnavailableReason;
export const DocumentViewBodyUnavailableReason = {
  REQUIRE_PASSWORD: 'REQUIRE_PASSWORD',
  REQUIRE_IDENTITY_VERIFICATION: 'REQUIRE_IDENTITY_VERIFICATION',
  REQUIRE_MINIMUM_AGE: 'REQUIRE_MINIMUM_AGE',
} as const;

export type PostViewBodyUnavailableReason = keyof typeof PostViewBodyUnavailableReason;
export const PostViewBodyUnavailableReason = {
  REQUIRE_PASSWORD: 'REQUIRE_PASSWORD',
  REQUIRE_IDENTITY_VERIFICATION: 'REQUIRE_IDENTITY_VERIFICATION',
  REQUIRE_MINIMUM_AGE: 'REQUIRE_MINIMUM_AGE',
} as const;

export type RedirectType = keyof typeof RedirectType;
export const RedirectType = {
  SLUG: 'SLUG',
  PERMALINK: 'PERMALINK',
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
  DOCUMENT: 'DOCUMENT',
  FOLDER: 'FOLDER',
} as const;

export type SingleSignOnProvider = keyof typeof SingleSignOnProvider;
export const SingleSignOnProvider = {
  APPLE: 'APPLE',
  GOOGLE: 'GOOGLE',
  KAKAO: 'KAKAO',
  NAVER: 'NAVER',
} as const;

export type SiteDateDisplay = keyof typeof SiteDateDisplay;
export const SiteDateDisplay = {
  NONE: 'NONE',
  CREATED_AT: 'CREATED_AT',
  UPDATED_AT: 'UPDATED_AT',
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

export type TextReplacementState = keyof typeof TextReplacementState;
export const TextReplacementState = {
  ACTIVE: 'ACTIVE',
  DISABLED: 'DISABLED',
} as const;

export type UserState = keyof typeof UserState;
export const UserState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

export type BillingKeyType = keyof typeof BillingKeyType;
export const BillingKeyType = {
  CARD: 'CARD',
  KAKAOPAY: 'KAKAOPAY',
} as const;

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
  DIVIDER: 'DIVIDER',
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
  FALLBACK: 'FALLBACK',
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

export type InAppPurchaseRecordState = keyof typeof InAppPurchaseRecordState;
export const InAppPurchaseRecordState = {
  PAID: 'PAID',
  REFUNDED: 'REFUNDED',
} as const;

export type InAppPurchaseStore = keyof typeof InAppPurchaseStore;
export const InAppPurchaseStore = {
  APP_STORE: 'APP_STORE',
  GOOGLE_PLAY: 'GOOGLE_PLAY',
} as const;

export type NoteStatus = keyof typeof NoteStatus;
export const NoteStatus = {
  OPEN: 'OPEN',
  RESOLVED: 'RESOLVED',
} as const;

export type NoteState = keyof typeof NoteState;
export const NoteState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
  DELETED_CASCADED: 'DELETED_CASCADED',
} as const;

export type DocumentCommentThreadState = keyof typeof DocumentCommentThreadState;
export const DocumentCommentThreadState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type DocumentCommentState = keyof typeof DocumentCommentState;
export const DocumentCommentState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type PaymentInvoiceState = keyof typeof PaymentInvoiceState;
export const PaymentInvoiceState = {
  UPCOMING: 'UPCOMING',
  PAID: 'PAID',
  OVERDUE: 'OVERDUE',
  CANCELED: 'CANCELED',
  WAIVED: 'WAIVED',
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

export type DocumentExportFormat = keyof typeof DocumentExportFormat;
export const DocumentExportFormat = {
  DOCX: 'DOCX',
  EPUB: 'EPUB',
  HWP: 'HWP',
  PDF: 'PDF',
} as const;

export type DocumentConflictKind = keyof typeof DocumentConflictKind;
export const DocumentConflictKind = {
  ATTRIBUTE: 'ATTRIBUTE',
  TEXT: 'TEXT',
  LIFECYCLE: 'LIFECYCLE',
  POSITION: 'POSITION',
  ORDER: 'ORDER',
} as const;

export type DocumentBundleKind = keyof typeof DocumentBundleKind;
export const DocumentBundleKind = {
  PUSHED: 'PUSHED',
  CONSOLIDATED: 'CONSOLIDATED',
  BASELINE: 'BASELINE',
} as const;

export type DocumentHeadKind = keyof typeof DocumentHeadKind;
export const DocumentHeadKind = {
  NORMAL: 'NORMAL',
  ISOLATED: 'ISOLATED',
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

export type UserDevicePlatform = keyof typeof UserDevicePlatform;
export const UserDevicePlatform = {
  IOS: 'IOS',
  ANDROID: 'ANDROID',
  WEB: 'WEB',
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

export type LlmAnalysisRunState = keyof typeof LlmAnalysisRunState;
export const LlmAnalysisRunState = {
  RUNNING: 'RUNNING',
  COMPLETED: 'COMPLETED',
  ABORTED: 'ABORTED',
  FAILED: 'FAILED',
} as const;

export type LlmCallState = keyof typeof LlmCallState;
export const LlmCallState = {
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
  ABORTED: 'ABORTED',
} as const;

export type PrismWorkflowState = keyof typeof PrismWorkflowState;
export const PrismWorkflowState = {
  RUNNING: 'RUNNING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
  CANCELED: 'CANCELED',
} as const;

export type PrismReviewRoundState = keyof typeof PrismReviewRoundState;
export const PrismReviewRoundState = {
  PENDING: 'PENDING',
  RUNNING: 'RUNNING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
  CANCELED: 'CANCELED',
} as const;

export type PrismReviewTier = keyof typeof PrismReviewTier;
export const PrismReviewTier = {
  LOW: 'LOW',
  MEDIUM: 'MEDIUM',
  HIGH: 'HIGH',
} as const;

export type PrismReviewPass = keyof typeof PrismReviewPass;
export const PrismReviewPass = {
  JUDGMENT: 'JUDGMENT',
  STYLISTIC: 'STYLISTIC',
} as const;

export type PrismReviewThreadState = keyof typeof PrismReviewThreadState;
export const PrismReviewThreadState = {
  OPEN: 'OPEN',
  CLOSED: 'CLOSED',
  RESOLVED: 'RESOLVED',
  WITHDRAWN: 'WITHDRAWN',
} as const;

export type PrismReviewCommentAuthor = keyof typeof PrismReviewCommentAuthor;
export const PrismReviewCommentAuthor = {
  USER: 'USER',
  AI: 'AI',
} as const;

export type PrismReaction = keyof typeof PrismReaction;
export const PrismReaction = {
  UP: 'UP',
  DOWN: 'DOWN',
} as const;

export type PrismRunState = keyof typeof PrismRunState;
export const PrismRunState = {
  RUNNING: 'RUNNING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
  CANCELED: 'CANCELED',
} as const;

export type PrismTurnState = keyof typeof PrismTurnState;
export const PrismTurnState = {
  IDLE: 'IDLE',
  ACTIVE: 'ACTIVE',
} as const;

export type PrismToolPhase = keyof typeof PrismToolPhase;
export const PrismToolPhase = {
  EXECUTED: 'EXECUTED',
  REJECTED: 'REJECTED',
} as const;

export type PrismToolRequestStatus = keyof typeof PrismToolRequestStatus;
export const PrismToolRequestStatus = {
  PENDING: 'PENDING',
  RESOLVED: 'RESOLVED',
  CLOSED: 'CLOSED',
} as const;

export type PrismToolPolicy = keyof typeof PrismToolPolicy;
export const PrismToolPolicy = {
  READ_ONLY: 'READ_ONLY',
  STANDARD: 'STANDARD',
  FULL: 'FULL',
} as const;

export type PrismToolResolver = keyof typeof PrismToolResolver;
export const PrismToolResolver = {
  USER: 'USER',
  SERVER: 'SERVER',
} as const;

export type PrismCreditEntryKind = keyof typeof PrismCreditEntryKind;
export const PrismCreditEntryKind = {
  GRANT: 'GRANT',
  TRIAL: 'TRIAL',
  REVIEW_CHARGE: 'REVIEW_CHARGE',
  REVIEW_REFUND: 'REVIEW_REFUND',
  CHAT_CHARGE: 'CHAT_CHARGE',
  ADJUSTMENT: 'ADJUSTMENT',
  PURCHASE: 'PURCHASE',
  BONUS: 'BONUS',
  REFUND_OUT: 'REFUND_OUT',
  EXPIRE: 'EXPIRE',
} as const;

export type PrismCreditPack = keyof typeof PrismCreditPack;
export const PrismCreditPack = {
  P100: 'P100',
  P330: 'P330',
  P690: 'P690',
  P1440: 'P1440',
  P3000: 'P3000',
} as const;

export type PrismCreditPurchaseChannel = keyof typeof PrismCreditPurchaseChannel;
export const PrismCreditPurchaseChannel = {
  BILLING_KEY: 'BILLING_KEY',
} as const;

export type PrismCreditPurchaseState = keyof typeof PrismCreditPurchaseState;
export const PrismCreditPurchaseState = {
  PENDING: 'PENDING',
  PAID: 'PAID',
  FAILED: 'FAILED',
} as const;

export type PrismCreditRefundKind = keyof typeof PrismCreditRefundKind;
export const PrismCreditRefundKind = {
  WITHDRAWAL: 'WITHDRAWAL',
  REMAINDER: 'REMAINDER',
} as const;

export type PrismCreditRefundMethod = keyof typeof PrismCreditRefundMethod;
export const PrismCreditRefundMethod = {
  PG_CANCEL: 'PG_CANCEL',
  MANUAL: 'MANUAL',
} as const;

export type PrismCreditRefundState = keyof typeof PrismCreditRefundState;
export const PrismCreditRefundState = {
  PENDING: 'PENDING',
  DONE: 'DONE',
} as const;

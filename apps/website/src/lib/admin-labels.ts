import type {
  EntityAvailability,
  EntityState,
  EntityType,
  EntityVisibility,
  InAppPurchaseStore,
  PaymentInvoiceState,
  PaymentOutcome,
  SubscriptionState,
  UserDevicePlatform,
  UserRole,
  UserState,
} from '@typie/lib/enums';

export type AdminTone = 'neutral' | 'brand' | 'success' | 'warning' | 'danger';

export const userStateLabels: Record<UserState, string> = {
  ACTIVE: '활성',
  DEACTIVATED: '비활성',
};

export const userStateTones: Record<UserState, AdminTone> = {
  ACTIVE: 'success',
  DEACTIVATED: 'danger',
};

export const userRoleLabels: Record<UserRole, string> = {
  ADMIN: '어드민',
  USER: '일반',
};

export const userRoleTones: Record<UserRole, AdminTone> = {
  ADMIN: 'warning',
  USER: 'neutral',
};

export const userDevicePlatformLabels: Record<UserDevicePlatform, string> = {
  IOS: 'iOS',
  ANDROID: 'Android',
  WEB: '웹',
};

export const entityTypeLabels: Record<EntityType, string> = {
  DOCUMENT: '문서',
  FOLDER: '폴더',
  POST: '포스트',
};

export const entityStateLabels: Record<EntityState, string> = {
  ACTIVE: '활성',
  DELETED: '삭제됨',
  PURGED: '완전 삭제',
};

export const entityStateTones: Record<EntityState, AdminTone> = {
  ACTIVE: 'success',
  DELETED: 'warning',
  PURGED: 'danger',
};

export const entityVisibilityLabels: Record<EntityVisibility, string> = {
  PUBLIC: '공개',
  UNLISTED: '링크가 있는 사람',
  PRIVATE: '비공개',
};

export const entityAvailabilityLabels: Record<EntityAvailability, string> = {
  UNLISTED: '링크가 있는 사람',
  PRIVATE: '나만',
};

export const subscriptionStateLabels: Record<SubscriptionState, string> = {
  ACTIVE: '활성',
  WILL_ACTIVATE: '활성 예정',
  WILL_EXPIRE: '만료 예정',
  IN_GRACE_PERIOD: '유예 기간',
  EXPIRED: '만료됨',
};

export const subscriptionStateTones: Record<SubscriptionState, AdminTone> = {
  ACTIVE: 'success',
  WILL_ACTIVATE: 'warning',
  WILL_EXPIRE: 'warning',
  IN_GRACE_PERIOD: 'danger',
  EXPIRED: 'neutral',
};

export const paymentInvoiceStateLabels: Record<PaymentInvoiceState, string> = {
  UPCOMING: '예정',
  PAID: '결제 완료',
  OVERDUE: '연체',
  CANCELED: '취소됨',
  WAIVED: '면제됨',
};

export const paymentInvoiceStateTones: Record<PaymentInvoiceState, AdminTone> = {
  UPCOMING: 'warning',
  PAID: 'success',
  OVERDUE: 'danger',
  CANCELED: 'neutral',
  WAIVED: 'neutral',
};

export const paymentOutcomeLabels: Record<PaymentOutcome, string> = {
  SUCCESS: '성공',
  FAILURE: '실패',
};

export const inAppPurchaseStoreLabels: Record<InAppPurchaseStore, string> = {
  APP_STORE: 'App Store',
  GOOGLE_PLAY: 'Google Play',
};

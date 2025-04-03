export type FolderState = keyof typeof FolderState;
export const FolderState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type JobState = keyof typeof JobState;
export const JobState = {
  PENDING: 'PENDING',
  RUNNING: 'RUNNING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
} as const;

export type PaymentMethodState = keyof typeof PaymentMethodState;
export const PaymentMethodState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

export type PostState = keyof typeof PostState;
export const PostState = {
  ACTIVE: 'ACTIVE',
  DELETED: 'DELETED',
} as const;

export type PostContentSyncKind = keyof typeof PostContentSyncKind;
export const PostContentSyncKind = {
  UPDATE: 'UPDATE',
  VECTOR: 'VECTOR',
  AWARENESS: 'AWARENESS',
  HEARTBEAT: 'HEARTBEAT',
} as const;

export type PreorderPaymentState = keyof typeof PreorderPaymentState;
export const PreorderPaymentState = {
  PENDING: 'PENDING',
  COMPLETED: 'COMPLETED',
  FAILED: 'FAILED',
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

export type UserState = keyof typeof UserState;
export const UserState = {
  ACTIVE: 'ACTIVE',
  DEACTIVATED: 'DEACTIVATED',
} as const;

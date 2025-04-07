export type PlanId = keyof typeof PlanId;
export const PlanId = {
  PLUS: 'PL0PLUS',
} as const;

export const PostContentSyncMessageKind = {
  HEARTBEAT: 1,
  INIT: 11,
  UPDATE: 21,
  VECTOR: 22,
  AWARENESS: 31,
  PRESENCE: 32,
} as const;

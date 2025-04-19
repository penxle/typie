export type PlanId = keyof typeof PlanId;
export const PlanId = {
  PLUS: 'PL0PLUS',
} as const;

export const WsMessageKind = {
  ESTABLISH: 1,
  HEARTBEAT: 9,
};

export const PostDocumentSyncMessageKind = {
  INIT: 101,
  UPDATE: 111,
  VECTOR: 112,
  AWARENESS: 121,
  PRESENCE: 122,
} as const;

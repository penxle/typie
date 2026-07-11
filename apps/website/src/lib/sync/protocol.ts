import { Encoder } from 'cbor-x';

export const SUBPROTOCOL = 'typie-sync.v1';

export const CLOSE_AUTH_FAILED = 4001;
export const CLOSE_BACKPRESSURE = 4002;
export const CLOSE_PROTOCOL_ERROR = 4003;

export const compareStreamSeq = (a: string, b: string): number => {
  const [am, as] = a.split('-').map(BigInt);
  const [bm, bs] = b.split('-').map(BigInt);
  if (am !== bm) return am < bm ? -1 : 1;
  if (as !== bs) return as < bs ? -1 : 1;
  return 0;
};

export type SnapshotCursor = { rowId: string; seq: number; offset: number };

export type ClientMessage =
  | { t: 'hello'; ticket: string; clientId: string; capabilities: string[] }
  | { t: 'ping' }
  | { t: 'attach'; documentId: string; sinceSeq?: string; snapshotCursor?: SnapshotCursor }
  | { t: 'detach'; documentId: string }
  | { t: 'push'; id: string; documentId: string; changesets: Uint8Array }
  | { t: 'pull'; id: string; documentId: string; sinceSeq?: string };

export type ServerMessage =
  | { t: 'hello-ack'; capabilities: string[] }
  | { t: 'pong' }
  | { t: 'attach-ack'; documentId: string }
  | { t: 'snapshot-chunk'; documentId: string; rowId: string; seq: number; offset: number; bytes: Uint8Array }
  | { t: 'snapshot-end'; documentId: string; seq: string; heads: Uint8Array; durableHeads: Uint8Array }
  | { t: 'changesets'; documentId: string; seq: string; bundles: Uint8Array[]; heads: Uint8Array; durableHeads: Uint8Array }
  | { t: 'reload'; documentId: string }
  | { t: 'push-ack'; id: string; heads: Uint8Array; durableHeads: Uint8Array }
  | { t: 'pull-ack'; id: string; changesets: Uint8Array[]; seq: string; heads: Uint8Array; durableHeads: Uint8Array; needsReload: boolean }
  | { t: 'error'; scope: 'connection' | 'document' | 'request'; documentId?: string; id?: string; code: string; permanent: boolean };

const SERVER_MESSAGE_TYPES = new Set([
  'hello-ack',
  'pong',
  'attach-ack',
  'snapshot-chunk',
  'snapshot-end',
  'changesets',
  'reload',
  'push-ack',
  'pull-ack',
  'error',
]);

const encoder = new Encoder({ useRecords: false });

export const encodeClientMessage = (message: ClientMessage): Uint8Array => encoder.encode(message);

export const decodeServerMessage = (data: Uint8Array): ServerMessage | null => {
  let raw: unknown;
  try {
    raw = encoder.decode(data);
  } catch {
    return null;
  }
  if (typeof raw !== 'object' || raw === null) return null;
  const t = (raw as { t?: unknown }).t;
  if (typeof t !== 'string' || !SERVER_MESSAGE_TYPES.has(t)) return null;
  return raw as ServerMessage;
};

export class SyncRequestError extends Error {
  readonly code: string;
  readonly permanent: boolean;

  constructor(code: string, permanent: boolean) {
    super(`sync request failed: ${code}`);
    this.code = code;
    this.permanent = permanent;
  }
}

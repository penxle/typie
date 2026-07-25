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

export const ASSET_MESSAGE_MAX_ITEMS = 100;

export type AssetNonceItem = { id: string; nonce: string };

export type ClientMessage =
  | { t: 'hello'; ticket: string; clientId: string; capabilities: string[] }
  | { t: 'ping' }
  | { t: 'attach'; documentId: string; sinceSeq?: string; snapshotCursor?: SnapshotCursor }
  | { t: 'detach'; documentId: string }
  | { t: 'push'; id: string; documentId: string; changesets: Uint8Array }
  | { t: 'pull'; id: string; documentId: string; sinceSeq?: string }
  | { t: 'asset-pull'; documentId: string; requestId: string; ids: string[] }
  | { t: 'asset-heartbeat'; documentId: string; items: AssetNonceItem[] }
  | { t: 'asset-failed'; documentId: string; items: AssetNonceItem[] };

export type ReadyAssetPayload =
  | { type: 'image'; id: string; url: string; originalUrl: string; width: number; height: number; placeholder: string | null }
  | { type: 'file'; id: string; url: string; name: string; size: number }
  | {
      type: 'embed';
      id: string;
      url: string;
      title: string | null;
      description: string | null;
      thumbnailUrl: string | null;
      html: string | null;
    }
  | { type: 'archived'; id: string; content: string };

// 네 종류 모두 이 경로로 해석한다. lease(=`pending`)는 image/file에만 존재하므로 embed·archived는
// ready 아니면 missing이다.
export type AssetStateEntry =
  | { id: string; state: 'missing' }
  | { id: string; state: 'pending'; meta: { kind: 'image' | 'file'; name: string; size: number } }
  | { id: string; state: 'ready'; asset: ReadyAssetPayload };

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
  | { t: 'asset-state'; documentId: string; requestId: string; assets: AssetStateEntry[]; final: boolean }
  | { t: 'asset-changed'; documentId: string; ids: string[] }
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
  'asset-state',
  'asset-changed',
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

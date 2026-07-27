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
export const ASSET_STRING_MAX_LENGTH = 256;

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

export type ErrorScope = 'connection' | 'document' | 'request';

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

// 네 종류 모두 이 경로로 해석한다. lease(=`pending`)는 클라이언트가 바이트를 쥐고 있는 image/file에만
// 존재하므로 embed·archived는 ready 아니면 missing이다.
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
  | { t: 'error'; scope: ErrorScope; documentId?: string; id?: string; code: string; permanent: boolean };

const encoder = new Encoder({ useRecords: false });

export const encodeMessage = (message: ClientMessage | ServerMessage): Uint8Array => encoder.encode(message);

export const decodeRaw = (data: Uint8Array): unknown => encoder.decode(data);

export type DecodeResult = { ok: true; message: ClientMessage } | { ok: false; reason: 'malformed' | 'unknown'; type?: string };

const malformed: DecodeResult = { ok: false, reason: 'malformed' };

const isStr = (v: unknown): v is string => typeof v === 'string';
const isBytes = (v: unknown): v is Uint8Array => v instanceof Uint8Array;
const isStrArray = (v: unknown): v is string[] => Array.isArray(v) && v.every(isStr);
// asset-pull은 push와 달리 토큰 버킷이 없다 — 과대 문자열이 Redis MGET·Postgres inArray로 증폭되기 전에 막는다.
const isBoundedStr = (v: unknown): v is string => isStr(v) && v.length <= ASSET_STRING_MAX_LENGTH;
const isBoundedStrArray = (v: unknown): v is string[] => Array.isArray(v) && v.every(isBoundedStr);
const isCursor = (v: unknown): v is SnapshotCursor => {
  if (typeof v !== 'object' || v === null) return false;
  const c = v as Record<string, unknown>;
  // eslint-disable-next-line unicorn/prefer-number-is-safe-integer -- accept any integer, not only the safe range
  return isStr(c.rowId) && Number.isInteger(c.seq) && Number.isInteger(c.offset) && (c.offset as number) >= 0;
};
const isSeqCursor = (v: unknown): v is string => typeof v === 'string' && (v === '' || /^\d+-\d+$/.test(v));
const isNonceItem = (v: unknown): v is AssetNonceItem => {
  if (typeof v !== 'object' || v === null) return false;
  const item = v as Record<string, unknown>;
  return isBoundedStr(item.id) && isBoundedStr(item.nonce);
};
const isNonceItemArray = (v: unknown): v is AssetNonceItem[] => Array.isArray(v) && v.every(isNonceItem);
const isSized = (v: { length: number }): boolean => v.length > 0 && v.length <= ASSET_MESSAGE_MAX_ITEMS;

export const decodeClientMessage = (data: Uint8Array): DecodeResult => {
  let raw: unknown;
  try {
    raw = encoder.decode(data);
  } catch {
    return malformed;
  }
  if (typeof raw !== 'object' || raw === null) return malformed;
  const m = raw as Record<string, unknown>;
  switch (m.t) {
    case 'hello': {
      return isStr(m.ticket) && isStr(m.clientId) && isStrArray(m.capabilities)
        ? { ok: true, message: { t: 'hello', ticket: m.ticket, clientId: m.clientId, capabilities: m.capabilities } }
        : malformed;
    }
    case 'ping': {
      return { ok: true, message: { t: 'ping' } };
    }
    case 'attach': {
      if (!isStr(m.documentId)) return malformed;
      if (m.sinceSeq !== undefined && !isSeqCursor(m.sinceSeq)) return malformed;
      if (m.snapshotCursor !== undefined && !isCursor(m.snapshotCursor)) return malformed;
      return { ok: true, message: { t: 'attach', documentId: m.documentId, sinceSeq: m.sinceSeq, snapshotCursor: m.snapshotCursor } };
    }
    case 'detach': {
      return isStr(m.documentId) ? { ok: true, message: { t: 'detach', documentId: m.documentId } } : malformed;
    }
    case 'push': {
      return isStr(m.id) && isStr(m.documentId) && isBytes(m.changesets)
        ? { ok: true, message: { t: 'push', id: m.id, documentId: m.documentId, changesets: m.changesets } }
        : malformed;
    }
    case 'pull': {
      if (!isStr(m.id) || !isStr(m.documentId)) return malformed;
      if (m.sinceSeq !== undefined && !isSeqCursor(m.sinceSeq)) return malformed;
      return { ok: true, message: { t: 'pull', id: m.id, documentId: m.documentId, sinceSeq: m.sinceSeq } };
    }
    case 'asset-pull': {
      return isStr(m.documentId) && isStr(m.requestId) && isBoundedStrArray(m.ids) && isSized(m.ids)
        ? { ok: true, message: { t: 'asset-pull', documentId: m.documentId, requestId: m.requestId, ids: m.ids } }
        : malformed;
    }
    case 'asset-heartbeat':
    case 'asset-failed': {
      return isStr(m.documentId) && isNonceItemArray(m.items) && isSized(m.items)
        ? {
            ok: true,
            message: {
              t: m.t,
              documentId: m.documentId,
              items: m.items.map((item) => ({ id: item.id, nonce: item.nonce })),
            },
          }
        : malformed;
    }
    default: {
      return { ok: false, reason: 'unknown', type: isStr(m.t) ? m.t : undefined };
    }
  }
};

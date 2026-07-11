import { asc, eq, max } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, DocumentBundles, DocumentStates, first } from '#/db/index.ts';
import { wasm } from './wasm-ffi.ts';

// * Sync storage model
//
// The un-snapshotted changeset history lives in a single per-document Redis
// Stream — the ordered, shared, coherence-safe source of truth every API
// instance reads. Each push appends ONE entry (its changeset bundle), so a
// client's catch-up is an `O(missing)` `XRANGE` from its last-seen stream id
// (its "seq") rather than an `O(history)` graph rebuild.
//
//   stream:{id}    Redis Stream. fields: c=base64 bundle, u=userId, d=deviceId
//   collected:{id} last stream id folded into the persisted snapshot by collect
//   trimmed:{id}   set once trim is about to drop entries, before the XTRIM (no TTL)
//
// The persisted history (`DocumentBundles`, one row per collected bundle) backs
// text/search/full-load; it is fed from the stream, which is trimmed behind
// the collected cursor while retaining a catch-up window. `document_states.heads`
// is the durable frontier of that persisted history — a single-row PK lookup,
// not expensive enough to justify caching (and a cache would just reintroduce
// the staleness window it was meant to solve).

export const streamKey = (documentId: string) => `document:changesets:stream:${documentId}`;
export const collectedKey = (documentId: string) => `document:changesets:collected:${documentId}`;
export const liveKey = (documentId: string) => `document:changesets:live:${documentId}`;
export const trimmedKey = (documentId: string) => `document:changesets:trimmed:${documentId}`;

// Retain this much history behind the collected cursor so a briefly-
// disconnected client catches up incrementally; anyone older full-reloads (see
// `truncated`). Trimming is keyed to the collected cursor, never `MAXLEN`, so an
// un-collected backlog (e.g. worker outage) is never trimmed away — no data loss.
const RETAIN_WINDOW_MS = 30 * 60 * 1000;

export type StreamEntry = { seq: string; changeset: Uint8Array; userId: string; deviceId: string };

// Redis Stream ids are `<ms>-<seq>`; compare numerically part-by-part.
export const seqCompare = (a: string, b: string): number => {
  const [am, as] = a.split('-').map(BigInt);
  const [bm, bs] = b.split('-').map(BigInt);
  if (am !== bm) return am < bm ? -1 : 1;
  if (as !== bs) return as < bs ? -1 : 1;
  return 0;
};

type XRangeRow = [id: string, fields: string[]];

const parseEntries = (rows: XRangeRow[]): StreamEntry[] =>
  rows.map(([seq, fields]) => {
    const map = new Map<string, string>();
    for (let i = 0; i < fields.length; i += 2) {
      map.set(fields[i], fields[i + 1]);
    }
    return {
      seq,
      changeset: Uint8Array.fromBase64(map.get('c') ?? ''),
      userId: map.get('u') ?? '',
      deviceId: map.get('d') ?? '',
    };
  });

// Append one push bundle to the stream. Returns the assigned stream id (the seq
// broadcast to peers and returned to the pusher). An accurate confirmed-heads
// contract keeps a pusher from re-sending already-known changesets, so the
// stream stays duplicate-free without an explicit dedup index.
export const appendBundle = async (documentId: string, bundle: Uint8Array, userId: string, deviceId: string): Promise<string> => {
  // No MAXLEN: trimming is the collect job's responsibility (keyed to the
  // collected cursor) so un-collected entries are never dropped.
  const seq = await redis.xadd(streamKey(documentId), '*', 'c', bundle.toBase64(), 'u', userId, 'd', deviceId);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- xadd on a stream always returns an id
  return seq!;
};

// Read up to `count` entries strictly after `sinceSeq` — the collect job's
// bounded batch.
export const readStreamBatch = async (documentId: string, sinceSeq: string | null, count: number): Promise<StreamEntry[]> => {
  const start = sinceSeq ? `(${sinceSeq}` : '-';
  const rows = (await redis.xrange(streamKey(documentId), start, '+', 'COUNT', count)) as XRangeRow[];
  return parseEntries(rows);
};

// Trim history older than the catch-up window *behind the collected cursor*.
// Entries after the cursor (un-collected) are always newer than the floor, so
// they are never trimmed regardless of how far collect has fallen behind.
// The durable `trimmed:{id}` marker (no TTL) is set *before* the XTRIM, keyed
// on the oldest retained id predating the floor: a crash after the set is a
// safe false positive (one spurious reload), whereas set-after-removal could
// leave a trimmed stream unmarked forever and silently lose the dropped
// prefix. Approximate trim (`~`) deferring physical removal errs the same
// safe direction. A no-op trim (nothing yet old enough) leaves the marker
// unset, so callers can tell "collect has run" from "the stream was ever
// truncated" instead of conflating the two.
export const trimStream = async (documentId: string, collectedSeq: string): Promise<void> => {
  const collectedMs = Number(collectedSeq.split('-')[0]);
  const floorMs = collectedMs - RETAIN_WINDOW_MS;
  if (floorMs <= 0) return;
  const floorSeq = `${floorMs}-0`;
  const oldest = (await redis.xrange(streamKey(documentId), '-', '+', 'COUNT', 1)) as XRangeRow[];
  if (oldest.length > 0 && seqCompare(oldest[0][0], floorSeq) < 0) {
    await redis.set(trimmedKey(documentId), '1');
  }
  await redis.xtrim(streamKey(documentId), 'MINID', '~', floorSeq);
};

// Has `trimStream` ever dropped entries (or been about to, see above)? `false`
// also covers a document that was never collected — there is nothing to trim yet.
export const hasStreamBeenTrimmed = async (documentId: string): Promise<boolean> => {
  return (await redis.exists(trimmedKey(documentId))) === 1;
};

// All stream entries strictly after `sinceSeq` (exclusive). `truncated` is true
// when the client's cursor predates the retained window — entries it still needs
// were already trimmed, so the caller must full-reload instead of incrementally
// catching up.
export const readStreamSince = async (
  documentId: string,
  sinceSeq: string | null,
): Promise<{ entries: StreamEntry[]; tip: string | null; truncated: boolean }> => {
  const key = streamKey(documentId);
  const start = sinceSeq ? `(${sinceSeq}` : '-';
  const rows = (await redis.xrange(key, start, '+')) as XRangeRow[];
  const entries = parseEntries(rows);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
  const tip = entries.length > 0 ? entries.at(-1)!.seq : sinceSeq;

  let truncated = false;
  if (sinceSeq) {
    const oldest = (await redis.xrange(key, '-', '+', 'COUNT', 1)) as XRangeRow[];
    // Oldest retained id newer than the client's cursor ⇒ the entries between
    // were trimmed. (A caught-up client whose stream is empty is fine.)
    if (oldest.length > 0 && seqCompare(oldest[0][0], sinceSeq) > 0) {
      truncated = true;
    }
  }
  return { entries, tip, truncated };
};

export const streamTip = async (documentId: string): Promise<string | null> => {
  const rows = (await redis.xrevrange(streamKey(documentId), '+', '-', 'COUNT', 1)) as XRangeRow[];
  return rows.length > 0 ? rows[0][0] : null;
};

// The durable frontier of the persisted (collected) history — `null` only for
// a document with no `DocumentStates` row yet (no durable truth to report).
export const getDurableHeads = async (documentId: string): Promise<Uint8Array | null> => {
  const row = await db
    .select({ heads: DocumentStates.heads })
    .from(DocumentStates)
    .where(eq(DocumentStates.documentId, documentId))
    .then(first);
  return row?.heads ?? null;
};

// The stream id up to which the persisted snapshot already includes everything.
// A full-load client starts incremental sync here and pulls only the tail.
export const getCollectedSeq = async (documentId: string): Promise<string | null> => {
  return await redis.get(collectedKey(documentId));
};

// Live frontier of the full document (snapshot + full stream). Push computes it
// (it must, to give the pusher an accurate confirmed-heads that prevents it from
// re-broadcasting peers' changesets) and stores it here so the O(1) reads on the
// pull / catch-up / document-load paths never rebuild the graph.
export const setLiveHeads = async (documentId: string, heads: Uint8Array): Promise<void> => {
  await redis.set(liveKey(documentId), heads.toBase64());
};

// Advance the cached live frontier by one push bundle — `O(bundle)`, no graph
// read: a dot is a head iff nothing references it as a parent, so the new
// frontier is `(prev ∪ bundle ids) − bundle parents` (`host.update_heads`).
// Returns null on a cold cache (no frontier yet); the caller bootstraps once
// via the merged-graph path. A concurrent push can interleave between GET and
// SET; the folds are commutative and a momentarily-stale frontier self-heals
// as soon as the missing ops gain children — the same staleness window the
// previous rebuild-from-merged-graph write had.
export const advanceLiveHeads = async (documentId: string, bundle: Uint8Array): Promise<Uint8Array | null> => {
  const prev = await getLiveHeads(documentId);
  if (!prev) {
    return null;
  }
  const next = await wasm.use((host) => host.update_heads(prev, bundle));
  await setLiveHeads(documentId, next);
  return next;
};

export const getLiveHeads = async (documentId: string): Promise<Uint8Array | null> => {
  const b64 = await redis.get(liveKey(documentId));
  return b64 ? Uint8Array.fromBase64(b64) : null;
};

// The collected bundle history for a document, seq-ascending, concatenated
// as-is — no decode, no re-encode. Every row is a verbatim copy of a push
// payload the collect job already applied successfully.
export const loadBundleStream = async (documentId: string): Promise<Uint8Array> => {
  const rows = await db
    .select({ payload: DocumentBundles.payload })
    .from(DocumentBundles)
    .where(eq(DocumentBundles.documentId, documentId))
    .orderBy(asc(DocumentBundles.seq));
  const total = rows.reduce((n, r) => n + r.payload.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const r of rows) {
    out.set(r.payload, off);
    off += r.payload.length;
  }
  return out;
};

// CAS baseline for the collect job: the tx-time `max(seq)` must match this
// pre-fold read, or another collect run appended in between.
export const getMaxBundleSeq = async (documentId: string): Promise<number> => {
  const row = await db
    .select({ max: max(DocumentBundles.seq) })
    .from(DocumentBundles)
    .where(eq(DocumentBundles.documentId, documentId));
  return row[0]?.max ?? 0;
};

// The full current document = collected bundle history + the stream tail the
// collect job has not folded in yet. Used by the full-load `graph` query, by
// push to report accurate live heads, and by revert. Pure byte concatenation
// (no FFI call): the bundle history is only ever collect-applied entries, and
// the tail is a stream the pusher validated before append — readers here only
// need to tolerate apply-level causal-not-ready gaps, not decode failure.
export const readMergedGraph = async (documentId: string): Promise<Uint8Array> => {
  const persisted = await loadBundleStream(documentId);

  const collected = await redis.get(collectedKey(documentId));
  const { entries } = await readStreamSince(documentId, collected);
  if (entries.length === 0) return persisted;

  const total = persisted.length + entries.reduce((n, e) => n + e.changeset.length, 0);
  const out = new Uint8Array(total);
  out.set(persisted, 0);
  let off = persisted.length;
  for (const e of entries) {
    out.set(e.changeset, off);
    off += e.changeset.length;
  }
  return out;
};

// `[u32 LE count][for each: u32 LE len, bytes]` — matches the Rust
// `graph::decode_length_prefixed` reader used by `collect_fold`.
export const packLengthPrefixed = (blobs: Uint8Array[]): Uint8Array => {
  const total = 4 + blobs.reduce((sum, b) => sum + 4 + b.length, 0);
  const out = new Uint8Array(total);
  const view = new DataView(out.buffer);
  let pos = 0;
  view.setUint32(pos, blobs.length, true);
  pos += 4;
  for (const b of blobs) {
    view.setUint32(pos, b.length, true);
    pos += 4;
    out.set(b, pos);
    pos += b.length;
  }
  return out;
};

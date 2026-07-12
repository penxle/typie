import { TypieError } from '@typie/lib/errors';
import { and, asc, eq, gt } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, DocumentBundles, Documents, DocumentStates, Entities, first } from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import {
  advanceLiveHeads,
  appendBundle,
  getCollectedSeq,
  getDurableHeads,
  getLiveHeads,
  hasStreamBeenTrimmed,
  readMergedGraph,
  readStreamBatch,
  seqCompare,
  setLiveHeads,
  streamKey,
  streamTip,
} from '#/utils/changeset.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { hasActiveSubscription } from '#/utils/plan.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import type { SyncDeps } from './types.ts';

export const createProductionDeps = (): SyncDeps => ({
  consumeTicket: async (ticket) => {
    const raw = await redis.getdel(`user:ws:${ticket}`);
    if (!raw) return null;
    const { sessionId, userId, deviceId, bootstrapBypassKeyHash } = JSON.parse(raw);
    if (!sessionId || !userId || !deviceId) return null;
    return { sessionId, userId, deviceId, bootstrapBypassKeyHash };
  },

  checkDocumentAccess: async (userId, documentId) => {
    const doc = await db
      .select({ siteId: Entities.siteId })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(first);
    if (!doc) return 'forbidden';
    try {
      await assertSitePermission({ userId, siteId: doc.siteId });
    } catch (err) {
      if (err instanceof TypieError) return 'forbidden';
      throw err;
    }
    const state = await db
      .select({ documentId: DocumentStates.documentId })
      .from(DocumentStates)
      .where(eq(DocumentStates.documentId, documentId))
      .then(first);
    return state ? 'ok' : 'not_v2';
  },

  checkWritable: async (userId) => await hasActiveSubscription({ userId }),

  getCollectedSeq,

  readBundleRow: async (documentId, rowId) => {
    const row = await db
      .select({ id: DocumentBundles.id, seq: DocumentBundles.seq, payload: DocumentBundles.payload })
      .from(DocumentBundles)
      .where(and(eq(DocumentBundles.documentId, documentId), eq(DocumentBundles.id, rowId)))
      .then(first);
    return row ?? null;
  },

  readBundlesAfter: async (documentId, afterSeq, limit) =>
    db
      .select({ id: DocumentBundles.id, seq: DocumentBundles.seq, payload: DocumentBundles.payload })
      .from(DocumentBundles)
      .where(and(eq(DocumentBundles.documentId, documentId), gt(DocumentBundles.seq, afterSeq)))
      .orderBy(asc(DocumentBundles.seq))
      .limit(limit),

  readStreamBatch: async (documentId, sinceSeq, count) => {
    const entries = await readStreamBatch(documentId, sinceSeq, count);
    return entries.map((e) => ({ seq: e.seq, changeset: e.changeset }));
  },

  isStreamTruncated: async (documentId, sinceSeq) => {
    const rows = (await redis.xrange(streamKey(documentId), '-', '+', 'COUNT', 1)) as [string, string[]][];
    return rows.length > 0 && seqCompare(rows[0][0], sinceSeq) > 0;
  },

  hasStreamBeenTrimmed,

  streamTip,

  getLiveHeads,
  getDurableHeads,

  subscribeChangesets: (documentId) => pubsub.subscribe('document:changesets', documentId),

  peekOpsCount: (changesets) => wasm.use((host) => host.peek_changeset_ops_count(changesets)),

  appendBundle,
  advanceLiveHeads,

  bootstrapLiveHeads: async (documentId) => {
    const graph = await readMergedGraph(documentId);
    const heads = await wasm.use((host) => host.heads(graph));
    await setLiveHeads(documentId, heads);
    return heads;
  },

  publishChangesets: (documentId, event) => pubsub.publish('document:changesets', documentId, event),

  enqueueCollect: async (documentId) => {
    await enqueueJob('document:changesets:collect', documentId);
  },
});

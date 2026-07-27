import { EntityAvailability } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, asc, eq, gt, inArray } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentArchivedNodes,
  DocumentBundles,
  Documents,
  DocumentStates,
  Embeds,
  Entities,
  Files,
  first,
  Images,
} from '#/db/index.ts';
import { buildFileUrl, buildImageOriginalUrl, buildImageUrl } from '#/graphql/resolvers/blob.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { deleteBlobLeases, extendBlobLeases, readBlobLeases } from '#/utils/blob-lease.ts';
import {
  advanceLiveHeads,
  appendBundle,
  clearPresence,
  getCollectedSeq,
  getDurableHeads,
  getLiveHeads,
  hasActivePresence,
  hasStreamBeenTrimmed,
  markPresence,
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
import { scheduleSweepDue } from '#/utils/zombie-sweep.ts';
import { buildAssetStateEntries, groupAssetIds, toLeaseItems, toLeaseMap } from './assets.ts';
import type { ReadyAssetPayload } from './protocol.ts';
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
      .select({ siteId: Entities.siteId, availability: Entities.availability })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(first);
    if (!doc) return 'forbidden';
    if (doc.availability === EntityAvailability.PRIVATE) {
      try {
        await assertSitePermission({ userId, siteId: doc.siteId });
      } catch (err) {
        if (err instanceof TypieError) return 'forbidden';
        throw err;
      }
    }
    const state = await db
      .select({ documentId: DocumentStates.documentId })
      .from(DocumentStates)
      .where(eq(DocumentStates.documentId, documentId))
      .then(first);
    return state ? 'ok' : 'not_v2';
  },

  checkWritable: async (userId) => await hasActiveSubscription({ userId }),

  markWriterActive: async (userId) => {
    await redis.zadd('writers:active', Date.now(), userId);
  },

  markPresence,

  // On the last lease release the document becomes sweep-eligible once quiescence
  // elapses; register it so the due-cron can pick it up without a further edit.
  clearPresence: async (documentId, connectionId) => {
    await clearPresence(documentId, connectionId);
    if (!(await hasActivePresence(documentId))) {
      await scheduleSweepDue(documentId);
    }
  },

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

  // lease를 먼저 읽고 completed row를 마지막에 읽어, 겹치면 completed가 이긴다. 읽는 사이 finalize가
  // 커밋되는 창은 좁아질 뿐 사라지지 않으므로 최종 수렴은 무효화 push + 클라이언트 재-pull이 담당한다.
  resolveAssetStates: async (ids) => {
    const { imageIds, fileIds, embedIds, archivedIds, assetIds } = groupAssetIds(ids);

    const leaseById = toLeaseMap(assetIds, await readBlobLeases(assetIds));

    const [images, files, embeds, archivedNodes] = await Promise.all([
      imageIds.length === 0
        ? []
        : db
            .select({
              id: Images.id,
              format: Images.format,
              path: Images.path,
              originalPath: Images.originalPath,
              width: Images.width,
              height: Images.height,
              placeholder: Images.placeholder,
            })
            .from(Images)
            .where(inArray(Images.id, imageIds)),
      fileIds.length === 0
        ? []
        : db.select({ id: Files.id, name: Files.name, size: Files.size, path: Files.path }).from(Files).where(inArray(Files.id, fileIds)),
      embedIds.length === 0
        ? []
        : db
            .select({
              id: Embeds.id,
              url: Embeds.url,
              title: Embeds.title,
              description: Embeds.description,
              thumbnailUrl: Embeds.thumbnailUrl,
              html: Embeds.html,
            })
            .from(Embeds)
            .where(inArray(Embeds.id, embedIds)),
      archivedIds.length === 0
        ? []
        : db
            .select({ id: DocumentArchivedNodes.id, content: DocumentArchivedNodes.content })
            .from(DocumentArchivedNodes)
            .where(inArray(DocumentArchivedNodes.id, archivedIds)),
    ]);

    const readyById = new Map<string, ReadyAssetPayload>();
    for (const row of images) {
      readyById.set(row.id, {
        type: 'image',
        id: row.id,
        url: buildImageUrl(row),
        originalUrl: buildImageOriginalUrl(row),
        width: row.width,
        height: row.height,
        placeholder: row.placeholder,
      });
    }
    for (const row of files) {
      readyById.set(row.id, { type: 'file', id: row.id, url: buildFileUrl(row), name: row.name, size: row.size });
    }
    for (const row of embeds) {
      readyById.set(row.id, {
        type: 'embed',
        id: row.id,
        url: row.url,
        title: row.title,
        description: row.description,
        thumbnailUrl: row.thumbnailUrl,
        html: row.html,
      });
    }
    for (const row of archivedNodes) {
      readyById.set(row.id, { type: 'archived', id: row.id, content: row.content });
    }

    return buildAssetStateEntries(ids, readyById, leaseById);
  },

  extendAssetLeases: async (items, userId) => {
    await extendBlobLeases(toLeaseItems(items), userId);
  },

  clearAssetLeases: async (items, userId) => await deleteBlobLeases(toLeaseItems(items), userId),

  publishAssetChanged: (documentId, ids) => pubsub.publish('document:assets', documentId, { ids }),

  subscribeAssetEvents: (documentId) => pubsub.subscribe('document:assets', documentId),
});

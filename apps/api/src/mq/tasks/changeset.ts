import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { DocumentBundleKind } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, asc, count, eq, lte, max, sql } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentBundles,
  DocumentChangesetsDeadLetter,
  DocumentCharacterCountChanges,
  DocumentHeadContributors,
  DocumentHeads,
  Documents,
  DocumentStates,
  DocumentSweeps,
  Entities,
  firstOrThrow,
} from '#/db/index.ts';
import { Lock } from '#/lock.ts';
import { pubsub } from '#/pubsub.ts';
import {
  collectedKey,
  expireIdleStream,
  getMaxBundleSeq,
  loadBundleStream,
  packLengthPrefixed,
  readStreamBatch,
  trimStream,
} from '#/utils/changeset.ts';
import { calculateBlobSizeFromAssetIds, extractAssetIdsFromPlainDoc } from '#/utils/entity.ts';
import { SYSTEM_USER_ID } from '#/utils/system-actor.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { clearSweepDue, scheduleSweepDue, sweepDocument, sweepDueKey } from '#/utils/zombie-sweep.ts';
import { enqueueJob } from '../index.ts';
import { defineCron, defineJob } from '../types.ts';
import type { Dayjs } from 'dayjs';

const log = logger.getChild('mq');

const HEAD_BUCKET_SECONDS = 10 * 60;
const CONSOLIDATE_THRESHOLD = 64;

const headBucket = (ts: Dayjs): Dayjs => dayjs.unix(Math.floor(ts.unix() / HEAD_BUCKET_SECONDS) * HEAD_BUCKET_SECONDS);

class StaleCollectError extends Error {}
class StaleConsolidationError extends Error {}

const concatPayloads = (payloads: Uint8Array[]): Uint8Array => {
  const total = payloads.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of payloads) {
    out.set(p, off);
    off += p.length;
  }
  return out;
};

export const DocumentChangesetsCollectJob = defineJob('document:changesets:collect', async (documentId: string) => {
  const lock = new Lock(`document:changesets:${documentId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  let updated = false;
  let userVisibleChanged = false;
  let persistedHeads: Uint8Array | null = null;
  let stale = false;
  let shouldConsolidate = false;
  let totalityViolations = 0;

  try {
    const collected = await redis.get(collectedKey(documentId));
    const entries = await readStreamBatch(documentId, collected, 5);
    if (entries.length === 0) {
      await expireIdleStream(documentId, collected);
      return;
    }

    const observedMaxSeq = await getMaxBundleSeq(documentId);
    const existing = await loadBundleStream(documentId);

    const failed: { userId: string; deviceId: string; payload: Uint8Array; error: string }[] = [];
    const perUserDelta = new Map<string, number>();
    const contributorIds = new Set<string>();

    // One State build for the whole batch (amortized) with per-bundle character
    // counts — replaces the per-entry `from_changesets` rebuild that made collect
    // `O(tail × build)`.
    const packed = packLengthPrefixed(entries.map((e) => e.changeset));
    const fold = await wasm.use((host) => host.collect_fold(existing, packed));
    totalityViolations = fold.totality_violations;

    let mergedChanged = false;
    let prevCharacterCount = fold.base_char_count;
    for (const [i, entry] of entries.entries()) {
      const status = fold.statuses[i];
      const charCount = fold.char_counts[i];
      if (status === 'applied') {
        perUserDelta.set(entry.userId, (perUserDelta.get(entry.userId) ?? 0) + (charCount - prevCharacterCount));
        prevCharacterCount = charCount;
        if (entry.userId !== SYSTEM_USER_ID) {
          contributorIds.add(entry.userId);
          userVisibleChanged = true;
        }
        mergedChanged = true;
      } else if (status === 'failed') {
        failed.push({ userId: entry.userId, deviceId: entry.deviceId, payload: entry.changeset, error: 'changeset apply failed' });
      }
    }

    const result = mergedChanged
      ? {
          plain: fold.plain,
          text: fold.text,
          characterCount: fold.char_counts.at(-1) ?? fold.base_char_count,
          heads: fold.heads,
        }
      : null;

    if (result || failed.length > 0) {
      let imageIds: string[] = [];
      let fileIds: string[] = [];
      let blobSize = 0;
      let characterCount = 0;

      if (result) {
        const ids = extractAssetIdsFromPlainDoc(result.plain);
        imageIds = ids.imageIds;
        fileIds = ids.fileIds;
        blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);
        characterCount = result.characterCount;
      }

      const updatedAt = dayjs();

      await db.transaction(async (tx) => {
        lock.signal.throwIfAborted();

        const maxSeq = await tx
          .select({ max: max(DocumentBundles.seq) })
          .from(DocumentBundles)
          .where(eq(DocumentBundles.documentId, documentId))
          .then((rows) => rows[0]?.max ?? 0);
        if (maxSeq !== observedMaxSeq) {
          throw new StaleCollectError();
        }

        if (result) {
          let seq = maxSeq;
          for (const [i, entry] of entries.entries()) {
            if (fold.statuses[i] !== 'applied') {
              continue;
            }
            seq += 1;
            await tx.insert(DocumentBundles).values({
              documentId,
              seq,
              payload: entry.changeset,
            });

            if (entry.zombieDots && entry.zombieDots.length > 0) {
              await tx
                .insert(DocumentSweeps)
                .values({ documentId, streamSeq: entry.seq, zombieDots: entry.zombieDots })
                .onConflictDoNothing({ target: [DocumentSweeps.documentId, DocumentSweeps.streamSeq] });
            }
          }

          await tx
            .update(DocumentStates)
            .set({
              json: result.plain,
              text: result.text,
              characterCount,
              blobSize,
              heads: result.heads,
              lastBundleSeq: seq,
              projectionDegraded: fold.projection_degraded,
              updatedAt,
            })
            .where(eq(DocumentStates.documentId, documentId));
          if (userVisibleChanged) {
            await tx.update(Documents).set({ updatedAt }).where(eq(Documents.id, documentId));
          }
          updated = true;
          persistedHeads = result.heads;

          for (const [userId, net] of perUserDelta) {
            if (net === 0) {
              continue;
            }

            await tx
              .insert(DocumentCharacterCountChanges)
              .values({
                documentId,
                userId,
                bucket: updatedAt.startOf('hour'),
                additions: Math.max(net, 0),
                deletions: Math.max(-net, 0),
              })
              .onConflictDoUpdate({
                target: [
                  DocumentCharacterCountChanges.userId,
                  DocumentCharacterCountChanges.documentId,
                  DocumentCharacterCountChanges.bucket,
                ],
                set: {
                  additions: net > 0 ? sql`${DocumentCharacterCountChanges.additions} + ${net}` : undefined,
                  deletions: net < 0 ? sql`${DocumentCharacterCountChanges.deletions} + ${-net}` : undefined,
                },
              });
          }

          const headBucketAt = headBucket(updatedAt);
          const headRow = await tx
            .insert(DocumentHeads)
            .values({ documentId, bucket: headBucketAt, heads: result.heads, characterCount })
            .onConflictDoUpdate({
              target: [DocumentHeads.documentId, DocumentHeads.bucket],
              set: { heads: result.heads, characterCount, updatedAt },
            })
            .returning({ id: DocumentHeads.id })
            .then(firstOrThrow);

          for (const userId of contributorIds) {
            await tx
              .insert(DocumentHeadContributors)
              .values({ headId: headRow.id, userId })
              .onConflictDoNothing({ target: [DocumentHeadContributors.headId, DocumentHeadContributors.userId] });
          }
        }

        if (failed.length > 0) {
          await tx.insert(DocumentChangesetsDeadLetter).values(
            failed.map((f) => ({
              documentId,
              payload: f.payload,
              userId: f.userId,
              deviceId: f.deviceId,
              errorMessage: f.error,
            })),
          );
        }
      });

      if (failed.length > 0) {
        for (const f of failed) {
          Sentry.captureMessage(`changeset dead-lettered: documentId=${documentId} userId=${f.userId} error=${f.error}`);
        }
      }
    }

    // Advance the collected cursor past this batch (applied + duplicate + dead-lettered),
    // then trim history behind the cursor.
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- entries is non-empty (guarded above)
    const lastSeq = entries.at(-1)!.seq;
    await redis.set(collectedKey(documentId), lastSeq);
    await trimStream(documentId, lastSeq);

    const bundleCount = await db.$count(DocumentBundles, eq(DocumentBundles.documentId, documentId));
    shouldConsolidate = bundleCount > CONSOLIDATE_THRESHOLD;
  } catch (err) {
    if (err instanceof StaleCollectError) {
      stale = true;
    } else {
      Sentry.captureException(err);
      throw err;
    }
  } finally {
    await lock.release();
    // Re-enqueue only after the lock is released, or the fresh job's tryAcquire
    // races the still-held lock and is lost (see lock.ts:32).
    if (stale) {
      await enqueueJob('document:changesets:collect', documentId);
    }
  }

  if (shouldConsolidate) {
    await enqueueJob('document:changesets:consolidate', documentId, {
      deduplication: { id: `document:changesets:consolidate:${documentId}`, ttl: 60 * 1000 },
    });
  }

  if (stale) {
    return;
  }

  if (totalityViolations > 0) {
    Sentry.captureMessage('totality violation detected', {
      level: 'warning',
      extra: { documentId, totalityViolations },
    });
    await scheduleSweepDue(documentId);
  }

  const collectedNow = await redis.get(collectedKey(documentId));
  const remaining = await readStreamBatch(documentId, collectedNow, 1);
  if (remaining.length > 0) {
    await enqueueJob('document:changesets:collect', documentId);
  }

  if (updated) {
    // Snapshot-accuracy propagation (heads) fires on any applied batch, including
    // a system-only sweep — attached clients must observe the swept frontier.
    pubsub.publish('document:changesets', documentId, {
      target: '*',
      seq: '',
      changesets: [],
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      heads: persistedHeads!.toBase64(),
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      durableHeads: persistedHeads!.toBase64(),
    });

    // User-facing side effects skip system-only sweeps: zombies are invisible, so
    // re-index / preview / recency / usage notifications would be no-ops that a
    // large backfill turns into a job storm.
    if (userVisibleChanged) {
      const { siteId, entityId, userId } = await db
        .select({ siteId: Entities.siteId, entityId: Entities.id, userId: Entities.userId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, documentId))
        .then(firstOrThrow);

      pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
      pubsub.publish('user:usage:update', userId, null);

      await enqueueJob('search:index:document', documentId, {
        deduplication: { id: `search:index:document:${documentId}`, ttl: 60 * 1000 },
      });

      await enqueueJob('document:preview:invalidate', documentId, {
        deduplication: { id: `document:preview:invalidate:${documentId}`, ttl: 60 * 60 * 1000 },
      });

      await scheduleSweepDue(documentId);
    }
  }
});

export const DocumentChangesetsConsolidateJob = defineJob('document:changesets:consolidate', async (documentId: string) => {
  const lock = new Lock(`document:changesets:${documentId}`);
  const acquired = await lock.tryAcquire();
  if (!acquired) return;

  try {
    const rows = await db
      .select({ seq: DocumentBundles.seq, kind: DocumentBundles.kind, payload: DocumentBundles.payload })
      .from(DocumentBundles)
      .where(eq(DocumentBundles.documentId, documentId))
      .orderBy(asc(DocumentBundles.seq));
    if (rows.length < 2) return;

    if (rows.some((r, i) => i > 0 && r.kind === DocumentBundleKind.CONSOLIDATED)) {
      Sentry.captureMessage(`document_bundles invariant violation: non-prefix consolidated row documentId=${documentId}`);
      return;
    }

    const stream = concatPayloads(rows.map((r) => r.payload));
    const result = await wasm.use((host) => host.consolidate(stream));
    if (!result.payload || result.consumed < 2) return;
    const consolidatedPayload = result.payload;

    let acc = 0;
    let lastMergedIdx = -1;
    for (const [i, row] of rows.entries()) {
      acc += row.payload.length;
      if (acc === result.consumed_bytes) {
        lastMergedIdx = i;
        break;
      }
      if (acc > result.consumed_bytes) break;
    }
    if (lastMergedIdx < 0) {
      Sentry.captureMessage(`consolidation boundary mid-row (multi-envelope row?) documentId=${documentId}`);
      return;
    }

    const lastMergedSeq = rows[lastMergedIdx].seq;
    await db.transaction(async (tx) => {
      lock.signal.throwIfAborted();

      const prefix = await tx
        .select({ count: count(), max: max(DocumentBundles.seq) })
        .from(DocumentBundles)
        .where(and(eq(DocumentBundles.documentId, documentId), lte(DocumentBundles.seq, lastMergedSeq)))
        .then(firstOrThrow);
      if (prefix.count !== lastMergedIdx + 1 || prefix.max !== lastMergedSeq) {
        throw new StaleConsolidationError();
      }

      await tx.delete(DocumentBundles).where(and(eq(DocumentBundles.documentId, documentId), lte(DocumentBundles.seq, lastMergedSeq)));
      await tx.insert(DocumentBundles).values({
        documentId,
        seq: lastMergedSeq,
        kind: DocumentBundleKind.CONSOLIDATED,
        payload: consolidatedPayload,
      });
    });

    await scheduleSweepDue(documentId);
  } catch (err) {
    if (err instanceof StaleConsolidationError) return;
    Sentry.captureException(err);
    throw err;
  } finally {
    await lock.release();
  }
});

export const DocumentZombieSweepJob = defineJob(
  'document:zombies:sweep',
  async ({ documentId, dueScore }: { documentId: string; dueScore: number }) => {
    const result = await sweepDocument(documentId);
    if (result.deferred) {
      await scheduleSweepDue(documentId, { retry: true });
      return;
    }

    await clearSweepDue(documentId, dueScore);

    if (result.applied) {
      log.info('zombie sweep applied to {documentId}: runs={deleteRunCount} zombies={zombieCount}', {
        documentId,
        deleteRunCount: result.deleteRunCount,
        zombieCount: result.zombieDots.length,
      });
    }
  },
);

export const DocumentChangesetsScanCron = defineCron('document:changesets:scan', '* * * * *', async () => {
  const prefix = 'document:changesets:stream:';

  let cursor = '0';
  do {
    const [next, keys] = await redis.scan(cursor, 'MATCH', `${prefix}*`, 'COUNT', 500);
    cursor = next;

    await Promise.all(
      keys.map((key) => {
        const documentId = key.slice(prefix.length);
        return enqueueJob('document:changesets:collect', documentId, {
          deduplication: { id: `document:changesets:collect:${documentId}` },
        });
      }),
    );
  } while (cursor !== '0');
});

export const DocumentZombieSweepDueCron = defineCron('document:zombies:sweep-due', '* * * * *', async () => {
  const rows = await redis.zrangebyscore(sweepDueKey, '-inf', Date.now(), 'WITHSCORES');

  const enqueues: Promise<void>[] = [];
  for (let i = 0; i < rows.length; i += 2) {
    const documentId = rows[i];
    const dueScore = Number(rows[i + 1]);
    enqueues.push(
      enqueueJob(
        'document:zombies:sweep',
        { documentId, dueScore },
        { deduplication: { id: `document:zombies:sweep:${documentId}`, ttl: 60 * 1000 } },
      ),
    );
  }

  await Promise.all(enqueues);
});

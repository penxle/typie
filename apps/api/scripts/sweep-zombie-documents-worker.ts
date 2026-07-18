#!/usr/bin/env node

import { setTimeout as sleep } from 'node:timers/promises';
import { parentPort, workerData } from 'node:worker_threads';
import { asc, eq, gt } from 'drizzle-orm';
import { db, DocumentCommentThreads, DocumentStates } from '#/db/index.ts';
import { streamTip } from '#/utils/changeset.ts';
import { extractSelectionDots } from '#/utils/comment-selection.ts';
import { shardOf } from '#/utils/sweep-sharding.ts';
import { recordSweepVerified, sweepDocument } from '#/utils/zombie-sweep.ts';
import type { SweepCommentHit, SweepReportEntry } from '#/utils/sweep-sharding.ts';

process.env.SCRIPT = '1';

type WorkerData = {
  workerIndex: number;
  workers: number;
  dryRun: boolean;
  batch: number;
  startCursor: string;
  enqueueRate: number;
  staleIds: string[];
};

const { workerIndex, workers, dryRun, batch, startCursor, enqueueRate, staleIds } = workerData as WorkerData;

// Documents that carry a prior failed/deferred report entry; a clean re-scan of one must
// clear that stale entry (main drops the ids this worker reports as resolved).
const staleSet = new Set(staleIds);

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

// rate / workers per worker keeps the aggregate collect-enqueue rate under the ceiling.
const throttleActive = !dryRun && enqueueRate > 0;
const perWorkerRate = enqueueRate / workers;
let tokens = perWorkerRate;
let lastRefill = Date.now();

const refillTokens = (): void => {
  const now = Date.now();
  tokens = Math.min(perWorkerRate, tokens + ((now - lastRefill) / 1000) * perWorkerRate);
  lastRefill = now;
};

const throttleEnqueue = async (): Promise<void> => {
  refillTokens();
  tokens -= 1;
  while (tokens < 0) {
    await sleep(Math.min(1000, Math.ceil((-tokens / perWorkerRate) * 1000)));
    refillTokens();
  }
};

const findZombieAnchoredComments = async (documentId: string, zombieDots: string[]): Promise<SweepCommentHit[]> => {
  if (zombieDots.length === 0) {
    return [];
  }
  const zombieSet = new Set(zombieDots);

  const threads = await db
    .select({ id: DocumentCommentThreads.id, selection: DocumentCommentThreads.selection })
    .from(DocumentCommentThreads)
    .where(eq(DocumentCommentThreads.documentId, documentId));

  const hits: SweepCommentHit[] = [];
  for (const thread of threads) {
    const extraction = extractSelectionDots(thread.selection);
    if (extraction.kind === 'unrecognized') {
      hits.push({ documentId, threadId: thread.id, hitDots: null });
      continue;
    }
    const hitDots = extraction.dots.filter((dot) => zombieSet.has(dot));
    if (hitDots.length > 0) {
      hits.push({ documentId, threadId: thread.id, hitDots });
    }
  }
  return hits;
};

type DocOutcome = {
  entry: SweepReportEntry | null;
  hits: SweepCommentHit[];
  dirty: boolean;
  zombies: number;
  applied: boolean;
};

const processDocument = async (documentId: string): Promise<DocOutcome> => {
  try {
    // Capture the tip BEFORE the verifying read so the marker is <= the sequence proven
    // zombie-free: a push after verification then moves the tip past the marker, so readiness
    // re-checks (fail-safe) rather than over-skipping unverified content.
    const preTip = await streamTip(documentId);
    const result = await sweepDocument(documentId, { dryRun });
    if (result.deferred) {
      return { entry: { documentId, reason: 'deferred' }, hits: [], dirty: false, zombies: 0, applied: false };
    }

    const dirty = result.deleteRunCount > 0;
    const hits = dryRun && dirty ? await findZombieAnchoredComments(documentId, result.zombieDots) : [];

    let verified = false;
    let verifiedTip = preTip;
    if (dryRun) {
      verified = result.zombieDots.length === 0;
    } else if (result.applied) {
      // Apply moved the tip; re-derive (sweep is single-pass) and capture the post-apply tip
      // before the reconfirm read, keeping the same <= verified-sequence ordering.
      verifiedTip = await streamTip(documentId);
      const recheck = await sweepDocument(documentId, { dryRun: true });
      verified = recheck.zombieDots.length === 0;
    } else {
      verified = true;
    }
    if (verified) {
      await recordSweepVerified(documentId, verifiedTip);
    }

    if (throttleActive && result.applied) {
      await throttleEnqueue();
    }

    return { entry: null, hits, dirty, zombies: result.zombieDots.length, applied: result.applied };
  } catch (err) {
    return {
      entry: { documentId, reason: 'failed', message: err instanceof Error ? err.message : String(err) },
      hits: [],
      dirty: false,
      zombies: 0,
      applied: false,
    };
  }
};

const run = async (): Promise<void> => {
  let cursor = startCursor;
  for (;;) {
    const rows = await db
      .select({ documentId: DocumentStates.documentId })
      .from(DocumentStates)
      .where(cursor ? gt(DocumentStates.documentId, cursor) : undefined)
      .orderBy(asc(DocumentStates.documentId))
      .limit(batch);
    if (rows.length === 0) {
      break;
    }

    const entries: SweepReportEntry[] = [];
    const hits: SweepCommentHit[] = [];
    const resolved: string[] = [];
    let scanned = 0;
    let dirty = 0;
    let zombies = 0;
    let applied = 0;

    for (const { documentId } of rows) {
      cursor = documentId;
      if (shardOf(documentId, workers) !== workerIndex) {
        continue;
      }
      scanned += 1;
      const outcome = await processDocument(documentId);
      if (outcome.entry) {
        entries.push(outcome.entry);
      } else if (staleSet.has(documentId)) {
        resolved.push(documentId);
      }
      if (outcome.hits.length > 0) {
        hits.push(...outcome.hits);
      }
      if (outcome.dirty) {
        dirty += 1;
        zombies += outcome.zombies;
      }
      if (outcome.applied) {
        applied += 1;
      }
    }

    // cursor = last visited id (any shard); restart resumes via `gt`, avoiding JS/DB collation risk.
    port.postMessage({ type: 'result', cursor, entries, hits, resolved, stats: { scanned, dirty, zombies, applied } });
  }

  port.postMessage({ type: 'exhausted' });
};

port.on('message', (message: { done?: boolean }) => {
  if (message.done) {
    process.exit(0);
  }
});

try {
  await run();
} catch (err) {
  port.postMessage({ type: 'fatal', message: err instanceof Error ? err.message : String(err) });
}

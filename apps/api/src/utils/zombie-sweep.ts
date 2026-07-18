import { redis } from '#/cache.ts';
import { Lock } from '#/lock.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { appendBundle, getDurableHeads, hasActivePresence, readMergedGraph, setLiveHeads, streamTip } from '#/utils/changeset.ts';
import { ensureSystemActor, SYSTEM_DEVICE_ID, SYSTEM_USER_ID } from '#/utils/system-actor.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import type { EditorServer } from '@typie/editor-ffi/server';

export type SweepResult = {
  deferred: boolean;
  zombieDots: string[];
  deleteRunCount: number;
  applied: boolean;
};

const QUIESCENCE_MS = 10 * 60 * 1000;
const RETRY_DELAY_MS = 5 * 60 * 1000;

export const sweepDueKey = 'document:sweep:due';

// field=documentId, value=<streamTip when confirmed zombie-free>. Kept out of
// sweepDocument so the sync-path semantics stay unchanged; written by the scripts.
export const sweepVerifiedKey = 'document:sweep:verified';

const SWEEP_VERIFIED_NO_TIP = 'none';

export const recordSweepVerified = async (documentId: string, tip: string | null): Promise<void> => {
  await redis.hset(sweepVerifiedKey, documentId, tip ?? SWEEP_VERIFIED_NO_TIP);
};

export const getSweepVerifiedMany = async (documentIds: string[]): Promise<(string | null)[]> =>
  documentIds.length === 0 ? [] : await redis.hmget(sweepVerifiedKey, ...documentIds);

export const sweepVerifiedMatchesTip = (marker: string | null, tip: string | null): boolean =>
  marker !== null && marker === (tip ?? SWEEP_VERIFIED_NO_TIP);

// host.sweep / host.zombie_dots come from the wasm-server bundle (Task 6); the
// intersection keeps this call site typed whether or not the regenerated .d.ts
// already declares them, and matches their signatures exactly when it does.
type SweepHost = EditorServer & {
  sweep: (graph: Uint8Array) => Uint8Array;
  zombie_dots: (graph: Uint8Array) => string[];
};

const computeSweep = (graph: Uint8Array): Promise<{ bundle: Uint8Array; deleteRunCount: number; zombieDots: string[] }> =>
  wasm.use((host) => {
    const h = host as SweepHost;
    const bundle = h.sweep(graph);
    const deleteRunCount = bundle.length > 0 ? h.peek_changeset_ops_count(bundle) : 0;
    const zombieDots = bundle.length > 0 ? h.zombie_dots(graph) : [];
    return { bundle, deleteRunCount, zombieDots };
  });

export const sweepDocument = async (documentId: string, opts?: { dryRun?: boolean }): Promise<SweepResult> => {
  if (opts?.dryRun) {
    const graph = await readMergedGraph(documentId);
    const { deleteRunCount, zombieDots } = await computeSweep(graph);
    return { deferred: false, zombieDots, deleteRunCount, applied: false };
  }

  const lock = new Lock(`document:changesets:${documentId}`);
  if (!(await lock.tryAcquire())) {
    return { deferred: true, zombieDots: [], deleteRunCount: 0, applied: false };
  }

  try {
    const tip = await streamTip(documentId);
    const tipRecent = tip !== null && Date.now() - Number(tip.split('-')[0]) < QUIESCENCE_MS;
    if (tipRecent || (await hasActivePresence(documentId))) {
      return { deferred: true, zombieDots: [], deleteRunCount: 0, applied: false };
    }

    const graph = await readMergedGraph(documentId);
    const { bundle, deleteRunCount, zombieDots } = await computeSweep(graph);
    if (deleteRunCount === 0) {
      return { deferred: false, zombieDots: [], deleteRunCount: 0, applied: false };
    }

    await ensureSystemActor();
    const seq = await appendBundle(documentId, bundle, SYSTEM_USER_ID, SYSTEM_DEVICE_ID, { zombieDots });

    const mergedGraph = await readMergedGraph(documentId);
    const heads = await wasm.use((host) => host.heads(mergedGraph));
    const durableHeads = (await getDurableHeads(documentId)) ?? new Uint8Array();

    await setLiveHeads(documentId, heads);

    pubsub.publish('document:changesets', documentId, {
      target: '*',
      seq,
      changesets: [bundle.toBase64()],
      heads: heads.toBase64(),
      durableHeads: durableHeads.toBase64(),
    });

    await enqueueJob('document:changesets:collect', documentId);

    return { deferred: false, zombieDots, deleteRunCount, applied: true };
  } finally {
    await lock.release();
  }
};

// Register a document for a future sweep pass keyed to `tip time + quiescence`.
// GT means an unchanged tip never moves the due earlier; a retry (fence/lock
// deferral, tip immovable without a new push) is pushed a fixed interval out so
// the minute-cron does not re-enqueue it every tick.
export const scheduleSweepDue = async (documentId: string, opts?: { retry?: boolean }): Promise<void> => {
  const tip = await streamTip(documentId);
  const tipDue = tip ? Number(tip.split('-')[0]) + QUIESCENCE_MS : Date.now();
  const due = opts?.retry ? Math.max(tipDue, Date.now() + RETRY_DELAY_MS) : tipDue;
  await redis.zadd(sweepDueKey, 'GT', due, documentId);
};

// Remove a due registration only when it still matches the score the cron
// observed — a newer registration (a push that arrived while the sweep ran)
// must survive so its document is not permanently skipped.
export const clearSweepDue = async (documentId: string, dueScore: number): Promise<void> => {
  const script = `
    local s = redis.call("zscore", KEYS[1], ARGV[1])
    if s and tonumber(s) == tonumber(ARGV[2]) then
      return redis.call("zrem", KEYS[1], ARGV[1])
    end
    return 0
  `;
  await redis.eval(script, 1, sweepDueKey, documentId, String(dueScore));
};

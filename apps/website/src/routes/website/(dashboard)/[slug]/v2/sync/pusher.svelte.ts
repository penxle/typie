import { SyncRequestError } from '$lib/sync/protocol';
import type { PusherOpts, PushResult, PushStatus } from './types';

const IDLE_MS = 500;
const MAX_WAIT_MS = 3000;
const BACKOFF_BASE_MS = 2000;
const BACKOFF_CAP_MS = 30_000;
const DORMANT_ADOPT_LIMIT = 8;

function isPermanent(err: unknown): boolean {
  if (err instanceof SyncRequestError) return err.permanent;
  return false;
}

export const isPermanentForTest = isPermanent;

export class Pusher {
  private readonly opts: PusherOpts;
  private confirmedHeads: Uint8Array;
  private durableHeads: Uint8Array;
  private capturedHeads: Uint8Array;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally non-reactive bookkeeping
  private readonly blockedCount = new Map<string, number>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally non-reactive bookkeeping
  private readonly dormant = new Set<string>();
  private inflight = false;
  private persistTail: Promise<void> = Promise.resolve();
  private persistQueued: Promise<void> | null = null;
  private pendingPush: Promise<void> | null = null;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- synchronous protection notifications
  private readonly protectionListeners = new Set<() => void>();
  private readonly resolveFlushWaiters: (() => void)[] = [];
  private flushAfterInflight = false;
  private idleTimer: ReturnType<typeof setTimeout> | null = null;
  private maxWaitTimer: ReturnType<typeof setTimeout> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;
  private readonly handleOnline = (): void => {
    this.retryNow();
  };

  status = $state<PushStatus>('idle');
  retryAttempt = $state(0);
  captureFailures = $state(0);
  pushFailed = $state(false);

  constructor(opts: PusherOpts) {
    this.opts = opts;
    this.confirmedHeads = opts.initialServerHeads;
    this.durableHeads = opts.initialDurableHeads;
    this.capturedHeads = opts.initialServerHeads;
    window.addEventListener('online', this.handleOnline);
    void this.firePush();
  }

  private localChangesetIds(): string[] {
    // Read the ids straight from the graph — O(#changesets). The old
    // `missingChangesetsFor(∅)` + `splitChangesets` walked, cloned, and re-encoded the
    // entire history on every capture/prune, the dominant per-tick sync cost on a large
    // document.
    return this.opts.editor.changesetIds();
  }

  private async capture(): Promise<void> {
    if (this.stopped) throw new Error('Pusher is stopped');
    const records = await this.opts.store.load(this.opts.documentId);
    if (this.stopped) throw new Error('Pusher is stopped');
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local-only, not reactive state
    const localAll = new Set(this.localChangesetIds());
    const adopted: Promise<number>[] = [];
    for (const rec of records) {
      if (localAll.has(rec.id)) {
        this.blockedCount.delete(rec.id);
        this.dormant.delete(rec.id);
        continue;
      }
      if (this.dormant.has(rec.id)) continue;
      const { ready } = this.opts.editor.partitionRemoteChangesets(rec.changeset);
      if (ready.length > 0) {
        adopted.push(this.opts.editor.receiveRemoteChangeset(ready));
        this.blockedCount.delete(rec.id);
      } else {
        const n = (this.blockedCount.get(rec.id) ?? 0) + 1;
        this.blockedCount.set(rec.id, n);
        if (n >= DORMANT_ADOPT_LIMIT) this.dormant.add(rec.id);
      }
    }
    if (adopted.length > 0) await Promise.all(adopted);

    await this.persistFresh();
  }

  // Crash durability is decoupled from the push cadence: `schedule()` starts a
  // persist immediately, so a refresh right after the last edit still restores
  // it from the delta store — only the push waits for the idle window. Runs are
  // chained (never concurrent) and a queued run subsumes later requests.
  private persistFresh(): Promise<void> {
    if (this.persistQueued) return this.persistQueued;
    const run = this.persistTail.then(async () => {
      this.persistQueued = null;
      if (this.stopped) throw new Error('Pusher is stopped');
      // Snapshot the frontier before computing the delta (both reads are
      // synchronous, so they agree), and advance `capturedHeads` only to that
      // snapshot. Reading the frontier after the awaited writes would swallow
      // ops applied while a write was in flight — they'd be past
      // `capturedHeads` without ever having been stored.
      const heads = this.opts.editor.currentHeads();
      const { bytes: fresh, withheld } = this.opts.editor.missingChangesetsFor(this.capturedHeads);
      if (fresh.length > 0) {
        for (const { id, bytes } of this.opts.editor.splitChangesets(fresh)) {
          await this.opts.store.put({ id, documentId: this.opts.documentId, changeset: bytes, createdAt: Date.now() });
        }
        this.opts.broadcast?.(fresh);
      }
      if (withheld > 0) {
        console.error(`Pusher: ${withheld} changeset(s) withheld from persist; retrying the same window next cycle`);
        this.opts.onEvent?.({ kind: 'persist.withheld', count: withheld });
      } else {
        this.capturedHeads = heads;
        this.captureFailures = 0;
        this.notifyProtectionChanged();
      }
    });
    // The chain must survive a failed run; `capturedHeads` is untouched on
    // failure, so the next persist retries the same delta.
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- failure is surfaced to the awaiting caller via `run`
    this.persistTail = run.catch(() => {});
    this.persistQueued = run;
    return run;
  }

  private async drain(): Promise<void> {
    if (this.pendingPush) {
      await this.pendingPush;
      return this.drain();
    }
    if (this.stopped) return;
    const { bytes: payload, withheld } = this.opts.editor.missingChangesetsFor(this.confirmedHeads);
    if (withheld > 0) this.opts.onEvent?.({ kind: 'persist.withheld', count: withheld });
    if (payload.length === 0) return;
    this.opts.onEvent?.({ kind: 'push.fired', bytes: payload.length });
    const pending = (async () => {
      const result: PushResult = await this.opts.pushFn(payload);
      this.setConfirmedHeads(result.heads);
      this.setDurableHeads(result.durableHeads);
    })();
    this.pendingPush = pending;
    try {
      await pending;
    } catch (err) {
      if (!this.stopped) {
        this.pushFailed = true;
        this.notifyProtectionChanged();
      }
      throw err;
    } finally {
      if (this.pendingPush === pending) this.pendingPush = null;
    }
  }

  private async prune(): Promise<void> {
    if (this.stopped) return;
    const { bytes: missing, withheld } = this.opts.editor.missingChangesetsFor(this.durableHeads);
    if (withheld > 0) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local-only, not reactive state
    const localAll = new Set(this.localChangesetIds());
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local-only, not reactive state
    const stillMissing = new Set(this.opts.editor.splitChangesets(missing).map((e) => e.id));
    const durableSet = localAll.difference(stillMissing);
    const records = await this.opts.store.load(this.opts.documentId);
    const toDelete = records.filter((r) => durableSet.has(r.id)).map((r) => r.id);
    await this.opts.store.deleteMany(this.opts.documentId, toDelete);
  }

  private clearTimers(): void {
    this.clearScheduleTimers();
    if (this.retryTimer) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
  }

  private clearScheduleTimers(): void {
    if (this.idleTimer) {
      clearTimeout(this.idleTimer);
      this.idleTimer = null;
    }
    if (this.maxWaitTimer) {
      clearTimeout(this.maxWaitTimer);
      this.maxWaitTimer = null;
    }
  }

  private flushScheduledChanges(): void {
    this.clearScheduleTimers();
    if (this.inflight) {
      this.flushAfterInflight = true;
      return;
    }
    if (this.status === 'retrying') return;
    void this.firePush();
  }

  private async firePush(): Promise<void> {
    if (this.stopped || this.status === 'error' || this.inflight) return;
    this.clearTimers();
    this.inflight = true;
    this.status = 'pushing';
    const startedAt = performance.now();
    try {
      let captureFailed = false;
      let captureError: unknown;
      try {
        await this.capture();
        this.captureFailures = 0;
      } catch (err) {
        captureFailed = true;
        captureError = err;
        this.captureFailures += 1;
        this.notifyProtectionChanged();
      }
      if (this.stopped) return;
      await this.drain();
      if (this.stopped) return;
      if (captureFailed) throw captureError;
      this.finishSuccess(startedAt);
    } catch (err) {
      if (this.stopped) return;
      this.opts.onEvent?.({ kind: 'push.error', message: String(err) });
      this.handleFailure(err);
    } finally {
      this.inflight = false;
      for (const resolve of this.resolveFlushWaiters) resolve();
      this.resolveFlushWaiters.length = 0;
      if (!this.stopped && this.flushAfterInflight) {
        this.flushAfterInflight = false;
        this.flushScheduledChanges();
      }
    }
  }

  private finishSuccess(startedAt: number): void {
    if (this.stopped) return;
    this.status = 'idle';
    this.retryAttempt = 0;
    this.opts.onEvent?.({ kind: 'push.success', durationMs: performance.now() - startedAt });
  }

  private handleFailure(err: unknown): void {
    if (isPermanent(err)) {
      this.status = 'error';
      console.error('Pusher: permanent failure', err);
      return;
    }
    this.status = 'retrying';
    this.retryAttempt += 1;
    const delay = Math.min(BACKOFF_BASE_MS * this.retryAttempt, BACKOFF_CAP_MS);
    console.warn(`Pusher: transient failure (attempt ${this.retryAttempt}), retrying in ${delay}ms`, err);
    this.retryTimer = setTimeout(() => {
      this.retryTimer = null;
      void this.firePush();
    }, delay);
  }

  private retryNow(): void {
    if (this.stopped) return;
    if (this.inflight) return;
    if (this.status === 'error') return;
    if (this.retryTimer) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
    this.status = 'idle';
    void this.firePush();
  }

  private notifyProtectionChanged(): void {
    if (this.stopped) return;
    for (const listener of this.protectionListeners) listener();
  }

  private coveredBy(heads: Uint8Array): boolean {
    const { bytes, withheld } = this.opts.editor.missingChangesetsFor(heads);
    return bytes.length === 0 && withheld === 0;
  }

  setConfirmedHeads(heads: Uint8Array): void {
    if (this.stopped) return;
    this.confirmedHeads = heads;
    if (this.isSynced()) this.pushFailed = false;
    this.notifyProtectionChanged();
  }

  isSynced(): boolean {
    return !this.stopped && this.coveredBy(this.confirmedHeads);
  }

  isProtected(): boolean {
    return !this.stopped && (this.coveredBy(this.capturedHeads) || this.isSynced());
  }

  onProtectionChange(listener: () => void): () => void {
    this.protectionListeners.add(listener);
    return () => this.protectionListeners.delete(listener);
  }

  async checkpoint(): Promise<void> {
    const requireProtection = () => {
      if (!this.isProtected()) throw new Error('Document changes are not protected');
    };
    if (this.isProtected()) return;
    // A blocked/failed local write must not prevent server acknowledgement from
    // protecting the same changes. The caller bounds its wait, not these writes.
    await Promise.any([this.captureNow().then(requireProtection), this.drain().then(requireProtection)]);
    requireProtection();
  }

  setDurableHeads(durableHeads: Uint8Array): void {
    if (this.stopped) return;
    this.durableHeads = durableHeads;
    void this.prune().catch((err) => {
      if (this.stopped) return;
      console.warn('Pusher: prune failed; will retry on the next durable-head update', err);
    });
  }

  async captureNow(): Promise<void> {
    try {
      await this.capture();
    } catch (err) {
      this.captureFailures += 1;
      this.notifyProtectionChanged();
      throw err;
    }
  }

  async flushNow(): Promise<void> {
    if (this.inflight) {
      await new Promise<void>((resolve) => {
        this.resolveFlushWaiters.push(resolve);
      });
    }
    await this.capture();
    await this.drain();
  }

  async pushNow(): Promise<void> {
    if (this.stopped) throw new Error('Pusher is stopped');
    const startedAt = performance.now();
    await this.drain();
    if (!this.isSynced()) throw new Error('Document changes are not confirmed by the server');
    this.finishSuccess(startedAt);
  }

  schedule(): void {
    if (this.stopped) return;
    // Before the error gate: a permanent push failure stops pushing, but local
    // crash durability must keep running — that is when it matters most.
    this.persistFresh().catch((err) => {
      this.captureFailures += 1;
      this.notifyProtectionChanged();
      console.warn('Pusher: persist failed, will retry on next edit', err);
    });
    if (this.status === 'error') return;

    if (this.idleTimer) clearTimeout(this.idleTimer);
    this.idleTimer = setTimeout(() => {
      this.idleTimer = null;
      this.flushScheduledChanges();
    }, IDLE_MS);

    if (!this.maxWaitTimer) {
      this.maxWaitTimer = setTimeout(() => {
        this.maxWaitTimer = null;
        this.flushScheduledChanges();
      }, MAX_WAIT_MS);
    }
  }

  stop(): void {
    this.stopped = true;
    this.clearTimers();
    window.removeEventListener('online', this.handleOnline);
    this.protectionListeners.clear();
  }
}

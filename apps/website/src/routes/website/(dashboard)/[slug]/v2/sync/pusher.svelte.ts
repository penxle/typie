import { isAggregatedError, isExchangeError, isGraphQLError } from '@mearie/svelte';
import { IndexeddbChangesetOutbox } from './outbox';
import type { Editor } from '$lib/editor-ffi/editor.svelte';
import type { ChangesetOutboxRecord, ChangesetOutboxStore } from './outbox';
import type { PusherEvent, PushStatus } from './types';

const IDLE_MS = 500;
const MAX_WAIT_MS = 3000;
const BACKOFF_BASE_MS = 2000;
const BACKOFF_CAP_MS = 30_000;

const PERMANENT_CODES = new Set(['invalid_changeset_payload']);

function isPermanent(err: unknown): boolean {
  if (!isAggregatedError(err)) return false;
  for (const e of err.errors) {
    if (isGraphQLError(e)) {
      const code = e.extensions?.code;
      if (typeof code === 'string' && PERMANENT_CODES.has(code)) return true;
    } else if (isExchangeError(e, 'http')) {
      const status = e.extensions?.statusCode;
      if (typeof status === 'number' && status >= 400 && status < 500) return true;
    }
  }
  return false;
}

type PusherOpts = {
  editor: Editor;
  documentId: string;
  clientId: string;
  initialServerHeads: Uint8Array;
  pushFn: (changesets: Uint8Array) => Promise<void>;
  outbox?: ChangesetOutboxStore;
  onEvent?: (event: PusherEvent) => void;
};

export class Pusher {
  status = $state<PushStatus>('idle');
  retryAttempt = $state(0);
  lastSentHeads: Uint8Array;

  private readonly opts: PusherOpts;
  private readonly outbox: ChangesetOutboxStore;
  private readonly ownsOutbox: boolean;
  private inflight = false;
  private capturePromise: Promise<void> | null = null;
  private captureAgain = false;
  private flushAfterInflight = false;
  private readonly drainingRecordIds: string[] = [];
  private idleTimer: ReturnType<typeof setTimeout> | null = null;
  private maxWaitTimer: ReturnType<typeof setTimeout> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;
  private readonly handleOnline = (): void => {
    this.retryNow();
  };

  constructor(opts: PusherOpts) {
    this.opts = opts;
    this.ownsOutbox = opts.outbox === undefined;
    this.outbox = opts.outbox ?? new IndexeddbChangesetOutbox();
    this.lastSentHeads = opts.initialServerHeads;
    window.addEventListener('online', this.handleOnline);
    void this.firePush();
  }

  schedule(): void {
    if (this.stopped) return;
    if (this.status === 'error') return;

    void this.capturePendingLocalChangesets().catch((err) => {
      console.warn('Pusher: failed to persist pending local changesets', err);
    });

    if (this.idleTimer) clearTimeout(this.idleTimer);
    this.idleTimer = setTimeout(() => {
      this.idleTimer = null;
      void this.flushScheduledChanges();
    }, IDLE_MS);

    if (!this.maxWaitTimer) {
      this.maxWaitTimer = setTimeout(() => {
        this.maxWaitTimer = null;
        void this.flushScheduledChanges();
      }, MAX_WAIT_MS);
    }
  }

  stop(): void {
    this.stopped = true;
    this.clearTimers();
    window.removeEventListener('online', this.handleOnline);
    if (!this.inflight && !this.capturePromise) {
      this.destroyOwnedOutbox();
    }
  }

  private destroyOwnedOutbox(): void {
    if (this.ownsOutbox) {
      this.outbox.destroy();
    }
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

  private async flushScheduledChanges(): Promise<void> {
    this.clearScheduleTimers();
    try {
      await this.capturePendingLocalChangesets();
    } catch (err) {
      this.opts.onEvent?.({ kind: 'push.error', message: String(err) });
      this.handleFailure(err);
      return;
    }
    if (this.inflight) {
      this.flushAfterInflight = true;
      return;
    }
    if (this.status === 'retrying') {
      return;
    }
    await this.firePush();
  }

  private async firePush(): Promise<void> {
    if (this.stopped) return;
    if (this.status === 'error') return;
    if (this.inflight) return;

    this.clearTimers();
    this.inflight = true;
    this.status = 'pushing';
    const startedAt = performance.now();

    try {
      await this.capturePendingLocalChangesets();
      if (this.stopped) return;

      const records = await this.outbox.load(this.opts.documentId);
      if (this.stopped) return;

      const drainedCount = await this.drainOutbox(records);
      if (this.stopped) return;

      this.finishSuccess(startedAt, drainedCount > 0);
      return;
    } catch (err) {
      if (this.stopped) return;
      this.opts.onEvent?.({ kind: 'push.error', message: String(err) });
      this.handleFailure(err);
      return;
    } finally {
      this.inflight = false;
      if (this.stopped) {
        this.destroyOwnedOutbox();
      } else if (this.flushAfterInflight) {
        this.flushAfterInflight = false;
        void this.flushScheduledChanges();
      }
    }
  }

  private applyRecordsLocally(records: ChangesetOutboxRecord[]): void {
    for (const record of records) {
      try {
        this.opts.editor.receiveRemoteChangeset(record.changesets);
      } catch (err) {
        console.warn('Pusher: failed to apply outbox changeset locally before resend', err);
      }
    }
  }

  private capturePendingLocalChangesets(): Promise<void> {
    if (this.stopped) return Promise.resolve();
    if (this.capturePromise) {
      this.captureAgain = true;
      return this.capturePromise;
    }

    this.capturePromise = this.runCapturePendingLocalChangesets();
    return this.capturePromise;
  }

  private async runCapturePendingLocalChangesets(): Promise<void> {
    try {
      do {
        this.captureAgain = false;
        const records = await this.outbox.load(this.opts.documentId);

        this.applyRecordsLocally(records);
        await this.captureCurrentSnapshot(records);
      } while (this.captureAgain && !this.stopped);
    } finally {
      this.capturePromise = null;
      if (this.stopped) {
        this.destroyOwnedOutbox();
      }
    }
  }

  private async captureCurrentSnapshot(records: ChangesetOutboxRecord[]): Promise<void> {
    const compactableRecords = records.filter(
      (record) => record.clientId === this.opts.clientId && !this.drainingRecordIds.includes(record.id),
    );
    const anchorRecord = compactableRecords.at(0);
    const before = anchorRecord?.baseHeads ?? records.at(-1)?.snapshotHeads ?? this.lastSentHeads;
    const snapshotHeads = new Uint8Array(this.opts.editor.currentHeads());
    if (bytesEqual(before, snapshotHeads)) return;

    const bundle = this.opts.editor.localChangesetsSince(before);
    if (bundle.length === 0) return;

    const record = {
      id: anchorRecord?.id ?? crypto.randomUUID(),
      documentId: this.opts.documentId,
      clientId: this.opts.clientId,
      baseHeads: new Uint8Array(before),
      snapshotHeads,
      changesets: new Uint8Array(bundle),
      createdAt: anchorRecord?.createdAt ?? Date.now(),
    };
    if (compactableRecords.length === 0) {
      await this.outbox.enqueue(record);
      return;
    }

    await this.outbox.replace(
      record,
      compactableRecords.map((record) => record.id),
    );
  }

  private async drainOutbox(records: ChangesetOutboxRecord[]): Promise<number> {
    let drainedCount = 0;
    for (const record of records) {
      if (!this.drainingRecordIds.includes(record.id)) {
        this.drainingRecordIds.push(record.id);
      }
    }
    try {
      for (const record of records) {
        if (this.stopped) return drainedCount;
        await this.pushRecord(record);
        drainedCount += 1;
      }
    } finally {
      for (const record of records) {
        const index = this.drainingRecordIds.indexOf(record.id);
        if (index !== -1) this.drainingRecordIds.splice(index, 1);
      }
    }
    return drainedCount;
  }

  private async pushRecord(record: ChangesetOutboxRecord): Promise<void> {
    this.opts.onEvent?.({ kind: 'push.fired', bytes: record.changesets.length });
    await this.opts.pushFn(record.changesets);
    await this.outbox.remove(record.id);
    this.lastSentHeads = record.snapshotHeads;
  }

  private finishSuccess(startedAt: number, emitEvent: boolean): void {
    if (this.stopped) return;
    this.inflight = false;
    this.status = 'idle';
    this.retryAttempt = 0;
    if (emitEvent) {
      this.opts.onEvent?.({ kind: 'push.success', durationMs: performance.now() - startedAt });
      this.schedule();
    }
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
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  return a.every((byte, index) => byte === b[index]);
}

import { isAggregatedError, isExchangeError, isGraphQLError } from '@mearie/svelte';
import type { Outbox } from './outbox.svelte';
import type { ClientCommitInput, DocumentObjectInput, OutboxEntry, PushStatus } from './types';

const IDLE_MS = 500;
const MAX_WAIT_MS = 3000;
const BACKOFF_BASE_MS = 2000;
const BACKOFF_CAP_MS = 30_000;

const PERMANENT_CODES = new Set([
  'object_hash_mismatch',
  'commit_hash_mismatch',
  'object_not_authorized',
  'invalid_parent_commit',
  'missing_root_object',
]);

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

type PushFn = (input: { documentId: string; commits: ClientCommitInput[]; objects: DocumentObjectInput[] }) => Promise<void>;

export type PusherEvent =
  | { kind: 'push.fired'; commits: number; objects: number }
  | { kind: 'push.success'; durationMs: number }
  | { kind: 'push.error'; message: string };

type PusherOpts = {
  documentId: string;
  outbox: Outbox;
  push: PushFn;
  onEvent?: (event: PusherEvent) => void;
};

export class Pusher {
  status = $state<PushStatus>('idle');
  retryAttempt = $state(0);

  private readonly opts: PusherOpts;
  private inflight = false;
  private idleTimer: ReturnType<typeof setTimeout> | null = null;
  private maxWaitTimer: ReturnType<typeof setTimeout> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  private readonly lastPushedHashes = new Set<string>();

  constructor(opts: PusherOpts) {
    this.opts = opts;
  }

  notifyEcho(commitHashes: string[]): void {
    for (const h of commitHashes) this.lastPushedHashes.delete(h);
  }

  notifyClear(): void {
    this.lastPushedHashes.clear();
  }

  async append(input: Omit<OutboxEntry, 'sequence'>): Promise<void> {
    try {
      await this.opts.outbox.append(input);
    } catch (err) {
      this.status = 'error';
      console.error('Pusher: IndexedDB append failed (permanent)', err);
      throw err;
    }
    this.schedule();
  }

  flushNow(): void {
    if (this.stopped) return;
    if (this.opts.outbox.isEmpty()) return;
    void this.firePush();
  }

  stop(): void {
    this.stopped = true;
    this.clearTimers();
  }

  schedule(): void {
    if (this.stopped) return;
    if (this.opts.outbox.isEmpty()) return;
    if (this.inflight) return;
    if (this.status === 'retrying' || this.status === 'error') return;

    if (this.idleTimer) clearTimeout(this.idleTimer);
    this.idleTimer = setTimeout(() => {
      this.idleTimer = null;
      void this.firePush();
    }, IDLE_MS);

    if (!this.maxWaitTimer) {
      this.maxWaitTimer = setTimeout(() => {
        this.maxWaitTimer = null;
        void this.firePush();
      }, MAX_WAIT_MS);
    }
  }

  private clearTimers(): void {
    if (this.idleTimer) {
      clearTimeout(this.idleTimer);
      this.idleTimer = null;
    }
    if (this.maxWaitTimer) {
      clearTimeout(this.maxWaitTimer);
      this.maxWaitTimer = null;
    }
    if (this.retryTimer) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
  }

  private async firePush(): Promise<void> {
    if (this.stopped) return;
    if (this.status === 'error') return;
    if (this.inflight) return;
    if (this.opts.outbox.isEmpty()) return;

    const allEntries = this.opts.outbox.entries;
    const entries =
      this.lastPushedHashes.size === 0 ? allEntries : allEntries.filter((e) => !this.lastPushedHashes.has(e.commit.commitHash));
    if (entries.length === 0) return;

    this.clearTimers();
    this.inflight = true;
    this.status = 'pushing';

    const commits = entries.map((e) => e.commit);
    const objects = entries.flatMap((e) => e.objects);

    this.opts.onEvent?.({ kind: 'push.fired', commits: commits.length, objects: objects.length });
    const startedAt = performance.now();

    try {
      await this.opts.push({ documentId: this.opts.documentId, commits, objects });
    } catch (err) {
      this.inflight = false;
      this.opts.onEvent?.({ kind: 'push.error', message: String(err) });
      this.handleFailure(err);
      return;
    }

    for (const c of commits) this.lastPushedHashes.add(c.commitHash);

    this.inflight = false;
    this.status = 'idle';
    this.retryAttempt = 0;
    this.opts.onEvent?.({ kind: 'push.success', durationMs: performance.now() - startedAt });
    this.schedule();
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
}

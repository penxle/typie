import { isAggregatedError, isExchangeError, isGraphQLError } from '@mearie/svelte';
import type { Editor } from '$lib/editor-ffi/editor.svelte';
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
  onEvent?: (event: PusherEvent) => void;
};

export class Pusher {
  status = $state<PushStatus>('idle');
  retryAttempt = $state(0);
  lastSentHeads: Uint8Array;

  private readonly opts: PusherOpts;
  private inflight = false;
  private idleTimer: ReturnType<typeof setTimeout> | null = null;
  private maxWaitTimer: ReturnType<typeof setTimeout> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;

  constructor(opts: PusherOpts) {
    this.opts = opts;
    this.lastSentHeads = opts.initialServerHeads;
  }

  schedule(): void {
    if (this.stopped) return;
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

  stop(): void {
    this.stopped = true;
    this.clearTimers();
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

    const before = this.lastSentHeads;
    const bundle = this.opts.editor.localChangesetsSince(before);
    if (bundle.length === 0) return;

    const snapshot = this.opts.editor.currentHeads();
    this.clearTimers();
    this.inflight = true;
    this.status = 'pushing';
    this.opts.onEvent?.({ kind: 'push.fired', bytes: bundle.length });
    const startedAt = performance.now();

    try {
      await this.opts.pushFn(bundle);
    } catch (err) {
      this.inflight = false;
      this.opts.onEvent?.({ kind: 'push.error', message: String(err) });
      this.handleFailure(err);
      return;
    }

    if (this.stopped) return;

    this.lastSentHeads = snapshot;
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

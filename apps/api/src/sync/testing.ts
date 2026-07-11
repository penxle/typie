import { compareStreamSeq } from './protocol.ts';
import type { ServerMessage } from './protocol.ts';
import type { BundleRow, ChangesetEvent, ChangesetSubscription, DocumentAccess, StreamEntry, SyncDeps, SyncSession } from './types.ts';

export class AsyncEventQueue<T> {
  #queue: T[] = [];
  #waiters: ((result: IteratorResult<T>) => void)[] = [];
  #done = false;

  push(value: T): void {
    if (this.#done) return;
    const waiter = this.#waiters.shift();
    if (waiter) waiter({ value, done: false });
    else this.#queue.push(value);
  }

  end(): void {
    this.#done = true;
    // eslint-disable-next-line unicorn/no-unnecessary-splice -- drain: splice return is iterated
    for (const waiter of this.#waiters.splice(0)) waiter({ value: undefined as never, done: true });
  }

  [Symbol.asyncIterator](): AsyncIterator<T> {
    return {
      next: (): Promise<IteratorResult<T>> => {
        if (this.#queue.length > 0) return Promise.resolve({ value: this.#queue.shift() as T, done: false });
        if (this.#done) return Promise.resolve({ value: undefined as never, done: true });
        return new Promise((resolve) => {
          this.#waiters.push(resolve);
        });
      },
    };
  }
}

export class FakeSyncDeps implements SyncDeps {
  #subscribers = new Map<string, Set<AsyncEventQueue<ChangesetEvent>>>();
  #seqCounter = 0;
  bundles = new Map<string, BundleRow[]>();
  stream = new Map<string, StreamEntry[]>();
  oldestRetained = new Map<string, string>();
  collectedSeq = new Map<string, string>();
  liveHeads = new Map<string, Uint8Array>();
  durableHeadsMap = new Map<string, Uint8Array>();
  tickets = new Map<string, SyncSession>();
  access = new Map<string, DocumentAccess>();
  invalidPayloads: Uint8Array[] = [];
  published: { documentId: string; event: ChangesetEvent }[] = [];
  collectJobs: string[] = [];

  consumeTicket = async (ticket: string): Promise<SyncSession | null> => {
    const session = this.tickets.get(ticket) ?? null;
    this.tickets.delete(ticket);
    return session;
  };

  checkDocumentAccess = async (_userId: string, documentId: string): Promise<DocumentAccess> => this.access.get(documentId) ?? 'ok';

  getCollectedSeq = async (documentId: string): Promise<string | null> => this.collectedSeq.get(documentId) ?? null;

  readBundleRow = async (documentId: string, rowId: string): Promise<BundleRow | null> =>
    (this.bundles.get(documentId) ?? []).find((r) => r.id === rowId) ?? null;

  readBundlesAfter = async (documentId: string, afterSeq: number, limit: number): Promise<BundleRow[]> =>
    (this.bundles.get(documentId) ?? []).filter((r) => r.seq > afterSeq).slice(0, limit);

  readStreamSince = async (documentId: string, sinceSeq: string | null) => {
    const all = this.stream.get(documentId) ?? [];
    const entries = sinceSeq === null ? [...all] : all.filter((e) => compareStreamSeq(e.seq, sinceSeq) > 0);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
    const tip = entries.length > 0 ? entries.at(-1)!.seq : sinceSeq;
    const oldest = this.oldestRetained.get(documentId);
    const truncated = sinceSeq !== null && oldest !== undefined && compareStreamSeq(oldest, sinceSeq) > 0;
    return { entries, tip, truncated };
  };

  readStreamBatch = async (documentId: string, sinceSeq: string | null, count: number): Promise<StreamEntry[]> => {
    const { entries } = await this.readStreamSince(documentId, sinceSeq);
    return entries.slice(0, count);
  };

  isStreamTruncated = async (documentId: string, sinceSeq: string): Promise<boolean> => {
    const oldest = this.oldestRetained.get(documentId);
    return oldest !== undefined && compareStreamSeq(oldest, sinceSeq) > 0;
  };

  streamTip = async (documentId: string): Promise<string | null> => {
    const entries = this.stream.get(documentId) ?? [];
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
    return entries.length > 0 ? entries.at(-1)!.seq : null;
  };

  getLiveHeads = async (documentId: string): Promise<Uint8Array | null> => this.liveHeads.get(documentId) ?? null;

  getDurableHeads = async (documentId: string): Promise<Uint8Array | null> => this.durableHeadsMap.get(documentId) ?? null;

  subscribeChangesets = (documentId: string): ChangesetSubscription => {
    const queue = new AsyncEventQueue<ChangesetEvent>();
    const set = this.#subscribers.get(documentId) ?? new Set();
    set.add(queue);
    this.#subscribers.set(documentId, set);
    return {
      [Symbol.asyncIterator]: () => queue[Symbol.asyncIterator](),
      return: () => {
        set.delete(queue);
        queue.end();
      },
    };
  };

  peekOpsCount = async (changesets: Uint8Array): Promise<number> => {
    if (this.invalidPayloads.some((p) => Buffer.compare(p, changesets) === 0)) throw new Error('invalid payload');
    return changesets.length === 0 ? 0 : 1;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars -- required by SyncDeps.appendBundle contract
  appendBundle = async (documentId: string, bundle: Uint8Array, _userId: string, _deviceId: string): Promise<string> => {
    const seq = `${++this.#seqCounter}-0`;
    const entries = this.stream.get(documentId) ?? [];
    entries.push({ seq, changeset: bundle });
    this.stream.set(documentId, entries);
    return seq;
  };

  advanceLiveHeads = async (documentId: string, bundle: Uint8Array): Promise<Uint8Array | null> => {
    const prev = this.liveHeads.get(documentId);
    if (!prev) return null;
    const next = Uint8Array.of(...prev, bundle.length % 256);
    this.liveHeads.set(documentId, next);
    return next;
  };

  bootstrapLiveHeads = async (documentId: string): Promise<Uint8Array> => {
    const heads = Uint8Array.of(0xb0);
    this.liveHeads.set(documentId, heads);
    return heads;
  };

  publishChangesets = (documentId: string, event: ChangesetEvent): void => {
    this.published.push({ documentId, event });
    this.emit(documentId, event);
  };

  enqueueCollect = async (documentId: string): Promise<void> => {
    this.collectJobs.push(documentId);
  };

  seedBundles(documentId: string, rows: BundleRow[]): void {
    this.bundles.set(documentId, rows);
  }

  seedStream(documentId: string, entries: StreamEntry[]): void {
    this.stream.set(documentId, entries);
    this.#seqCounter = Math.max(this.#seqCounter, ...entries.map((e) => Number(e.seq.split('-')[0])));
  }

  emit(documentId: string, event: ChangesetEvent): void {
    for (const queue of this.#subscribers.get(documentId) ?? []) queue.push(event);
  }

  subscriberCount(documentId: string): number {
    return this.#subscribers.get(documentId)?.size ?? 0;
  }

  trimTo(documentId: string, seq: string): void {
    this.oldestRetained.set(documentId, seq);
    this.stream.set(
      documentId,
      (this.stream.get(documentId) ?? []).filter((e) => compareStreamSeq(e.seq, seq) >= 0),
    );
  }
}

export const collectSend = (): { sent: ServerMessage[]; send: (message: ServerMessage) => Promise<void> } => {
  const sent: ServerMessage[] = [];
  return { sent, send: async (message) => void sent.push(message) };
};

import type { OutboxEntry } from './types';

const DB_NAME = 'typie-document-sync-outbox';
const DB_VERSION = 1;
const STORE = 'pendingEntries';
const INDEX = 'byDocSeq';

type StoredEntry = OutboxEntry & { documentId: string };

type ChannelMessage = { type: 'append'; entry: OutboxEntry } | { type: 'prune'; commitHashes: string[] } | { type: 'clear' };

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.addEventListener('upgradeneeded', () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE)) {
        const store = db.createObjectStore(STORE, { keyPath: ['documentId', 'commit.commitHash'] });
        store.createIndex(INDEX, ['documentId', 'sequence']);
      }
    });
    req.addEventListener('success', () => resolve(req.result));
    req.addEventListener('error', () => reject(req.error));
  });
}

export class Outbox {
  private readonly db: IDBDatabase;
  private readonly documentId: string;
  private readonly channel: BroadcastChannel;
  private mirror = $state<OutboxEntry[]>([]);
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  private readonly pending = new Set<string>();
  private nextSequence = 1;
  private writeQueue: Promise<unknown> = Promise.resolve();

  private constructor(db: IDBDatabase, documentId: string) {
    this.db = db;
    this.documentId = documentId;
    this.channel = new BroadcastChannel(`typie-document-sync-outbox-${documentId}`);
  }

  private onChannelMessage = (ev: MessageEvent<ChannelMessage>) => {
    const msg = ev.data;
    switch (msg.type) {
      case 'append': {
        if (this.pending.has(msg.entry.commit.commitHash)) return;
        this.pending.add(msg.entry.commit.commitHash);
        this.mirror = [...this.mirror, msg.entry].toSorted((a, b) => a.sequence - b.sequence);
        if (msg.entry.sequence >= this.nextSequence) this.nextSequence = msg.entry.sequence + 1;
        return;
      }
      case 'prune': {
        // eslint-disable-next-line svelte/prefer-svelte-reactivity
        const set = new Set(msg.commitHashes);
        for (const h of msg.commitHashes) this.pending.delete(h);
        this.mirror = this.mirror.filter((e) => !set.has(e.commit.commitHash));
        return;
      }
      case 'clear': {
        this.pending.clear();
        this.mirror = [];
        this.nextSequence = 1;
        return;
      }
      default: {
        msg satisfies never;
      }
    }
  };

  static async open(documentId: string): Promise<Outbox> {
    const db = await openDb();
    const outbox = new Outbox(db, documentId);
    await outbox.hydrateMirror();
    outbox.channel.addEventListener('message', outbox.onChannelMessage);
    return outbox;
  }

  close(): void {
    this.channel.removeEventListener('message', this.onChannelMessage);
    this.channel.close();
    this.db.close();
  }

  append(input: Omit<OutboxEntry, 'sequence'>): Promise<void> {
    return this.chain(() => this.writeAndMirror(input));
  }

  prune(commitHashes: string[]): Promise<void> {
    if (commitHashes.length === 0) return Promise.resolve();
    return this.chain(() => this.pruneInternal(commitHashes));
  }

  clear(): Promise<void> {
    return this.chain(() => this.clearInternal());
  }

  get entries(): readonly OutboxEntry[] {
    return this.mirror;
  }

  hasPending(commitHash: string): boolean {
    return this.pending.has(commitHash);
  }

  get pendingSize(): number {
    return this.pending.size;
  }

  isEmpty(): boolean {
    return this.mirror.length === 0;
  }

  firstParentCommitHash(): string | null {
    if (this.mirror.length === 0) return null;
    return this.mirror[0].commit.parentCommitHash;
  }

  private chain<T>(work: () => Promise<T>): Promise<T> {
    const next = this.writeQueue.then(work);
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    this.writeQueue = next.catch(() => {});
    return next;
  }

  private async writeAndMirror(input: Omit<OutboxEntry, 'sequence'>): Promise<void> {
    const entry: OutboxEntry = { ...input, sequence: this.nextSequence };
    await new Promise<void>((resolve, reject) => {
      const tx = this.db.transaction(STORE, 'readwrite');
      const stored: StoredEntry = { ...entry, documentId: this.documentId };
      tx.objectStore(STORE).put(stored);
      tx.addEventListener('complete', () => resolve());
      tx.addEventListener('error', () => reject(tx.error));
      tx.addEventListener('abort', () => reject(tx.error ?? new Error('Outbox append: transaction aborted')));
    });
    this.nextSequence += 1;
    this.pending.add(entry.commit.commitHash);
    this.mirror.push(entry);
    this.channel.postMessage({ type: 'append', entry });
  }

  private async pruneInternal(commitHashes: string[]): Promise<void> {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const set = new Set(commitHashes);
    await new Promise<void>((resolve, reject) => {
      const tx = this.db.transaction(STORE, 'readwrite');
      const store = tx.objectStore(STORE);
      for (const hash of commitHashes) {
        store.delete([this.documentId, hash]);
      }
      tx.addEventListener('complete', () => resolve());
      tx.addEventListener('error', () => reject(tx.error));
      tx.addEventListener('abort', () => reject(tx.error ?? new Error('Outbox prune: transaction aborted')));
    });
    for (const h of commitHashes) this.pending.delete(h);
    this.mirror = this.mirror.filter((e) => !set.has(e.commit.commitHash));
    this.channel.postMessage({ type: 'prune', commitHashes });
  }

  private async clearInternal(): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      const tx = this.db.transaction(STORE, 'readwrite');
      const store = tx.objectStore(STORE);
      const index = store.index(INDEX);
      const range = IDBKeyRange.bound([this.documentId, -Infinity], [this.documentId, Infinity]);
      const cursorReq = index.openKeyCursor(range);
      cursorReq.addEventListener('success', () => {
        const cursor = cursorReq.result;
        if (cursor) {
          store.delete(cursor.primaryKey);
          cursor.continue();
        }
      });
      tx.addEventListener('complete', () => resolve());
      tx.addEventListener('error', () => reject(tx.error));
      tx.addEventListener('abort', () => reject(tx.error ?? new Error('Outbox clear: transaction aborted')));
    });
    this.pending.clear();
    this.mirror = [];
    this.nextSequence = 1;
    this.channel.postMessage({ type: 'clear' });
  }

  private async hydrateMirror(): Promise<void> {
    const stored = await new Promise<StoredEntry[]>((resolve, reject) => {
      const tx = this.db.transaction(STORE, 'readonly');
      const idx = tx.objectStore(STORE).index(INDEX);
      const range = IDBKeyRange.bound([this.documentId, -Infinity], [this.documentId, Infinity]);
      const req = idx.getAll(range);
      req.addEventListener('success', () => resolve(req.result as StoredEntry[]));
      req.addEventListener('error', () => reject(req.error));
    });
    const sorted = stored
      .map((s) => ({ commit: s.commit, objects: s.objects, sequence: s.sequence }))
      .toSorted((a, b) => a.sequence - b.sequence);
    this.mirror = sorted;
    for (const e of sorted) this.pending.add(e.commit.commitHash);
    this.nextSequence = (sorted.at(-1)?.sequence ?? 0) + 1;
  }
}

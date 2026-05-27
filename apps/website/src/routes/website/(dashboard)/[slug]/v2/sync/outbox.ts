const DB_NAME = 'typie:changeset-outbox';
const DB_VERSION = 1;
const STORE_NAME = 'bundles';
const DOCUMENT_ID_INDEX = 'documentId';

export type ChangesetOutboxRecord = {
  id: string;
  documentId: string;
  clientId: string;
  baseHeads: Uint8Array;
  snapshotHeads: Uint8Array;
  changesets: Uint8Array;
  createdAt: number;
};

export type ChangesetOutboxStore = {
  load(documentId: string): Promise<ChangesetOutboxRecord[]>;
  enqueue(record: ChangesetOutboxRecord): Promise<void>;
  replace(record: ChangesetOutboxRecord, obsoleteIds: string[]): Promise<void>;
  remove(id: string): Promise<void>;
  destroy(): void;
};

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.addEventListener('error', () => reject(request.error));
    request.addEventListener('success', () => resolve(request.result));

    request.addEventListener('upgradeneeded', () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        const store = db.createObjectStore(STORE_NAME, { keyPath: 'id' });
        store.createIndex(DOCUMENT_ID_INDEX, DOCUMENT_ID_INDEX);
      }
    });
  });
}

function deleteDatabase(): Promise<void> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.deleteDatabase(DB_NAME);
    request.addEventListener('success', () => resolve());
    request.addEventListener('error', () => reject(request.error));
  });
}

function hasExpectedSchema(db: IDBDatabase): boolean {
  if (!db.objectStoreNames.contains(STORE_NAME)) return false;

  const transaction = db.transaction(STORE_NAME, 'readonly');
  const store = transaction.objectStore(STORE_NAME);
  return store.indexNames.contains(DOCUMENT_ID_INDEX);
}

export class IndexeddbChangesetOutbox implements ChangesetOutboxStore {
  #db: IDBDatabase | null = null;
  #destroyed = false;

  async #ensureDb(): Promise<IDBDatabase> {
    if (this.#db) return this.#db;
    let db = await openDatabase();

    if (!hasExpectedSchema(db)) {
      db.close();
      await deleteDatabase();
      db = await openDatabase();
    }

    this.#db = db;
    return db;
  }

  #request<T>(request: IDBRequest<T>): Promise<T> {
    return new Promise((resolve, reject) => {
      request.addEventListener('error', () => reject(request.error));
      request.addEventListener('success', () => resolve(request.result));
    });
  }

  #transaction(transaction: IDBTransaction): Promise<void> {
    return new Promise((resolve, reject) => {
      transaction.addEventListener('abort', () => reject(transaction.error));
      transaction.addEventListener('complete', () => resolve());
      transaction.addEventListener('error', () => reject(transaction.error));
    });
  }

  async load(documentId: string): Promise<ChangesetOutboxRecord[]> {
    if (this.#destroyed) return [];

    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME);
    const records = (await this.#request(store.index(DOCUMENT_ID_INDEX).getAll(documentId))) as ChangesetOutboxRecord[];
    return records.toSorted((a, b) => a.createdAt - b.createdAt);
  }

  async enqueue(record: ChangesetOutboxRecord): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    await this.#request(store.put(record));
    await this.#transaction(transaction);
  }

  async replace(record: ChangesetOutboxRecord, obsoleteIds: string[]): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    const requests = [
      this.#request(store.put(record)),
      ...obsoleteIds.filter((id) => id !== record.id).map((id) => this.#request(store.delete(id))),
    ];
    await Promise.all(requests);
    await this.#transaction(transaction);
  }

  async remove(id: string): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    await this.#request(store.delete(id));
    await this.#transaction(transaction);
  }

  destroy(): void {
    this.#destroyed = true;
    this.#db?.close();
    this.#db = null;
  }
}

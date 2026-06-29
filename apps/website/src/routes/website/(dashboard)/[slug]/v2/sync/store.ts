const DB_NAME = 'typie:changeset-delta';
const DB_VERSION = 1;
const STORE_NAME = 'changesets';
const DOCUMENT_ID_INDEX = 'documentId';

export type DeltaRecord = {
  id: string; // first-op dot "{actor}:{clock}" — DOCUMENT-LOCAL unique (deterministic seed '1:0' collides cross-doc!)
  documentId: string;
  changeset: Uint8Array;
  createdAt: number;
};

export type DeltaStore = {
  load(documentId: string): Promise<DeltaRecord[]>;
  put(record: DeltaRecord): Promise<void>;
  deleteMany(documentId: string, ids: string[]): Promise<void>; // documentId-scoped (round-4 CRIT)
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
        // 복합 PK [documentId, id]: seed dot '1:0'이 문서마다 결정적이라 'id' 단독이면 문서 간 덮어쓰기(round-4 CRIT).
        const store = db.createObjectStore(STORE_NAME, { keyPath: ['documentId', 'id'] });
        store.createIndex(DOCUMENT_ID_INDEX, DOCUMENT_ID_INDEX);
      }
    });
  });
}

export class IndexeddbDeltaStore implements DeltaStore {
  #db: IDBDatabase | null = null;
  #destroyed = false;

  async #ensureDb(): Promise<IDBDatabase> {
    if (this.#db) return this.#db;
    const db = await openDatabase();
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

  async load(documentId: string): Promise<DeltaRecord[]> {
    if (this.#destroyed) return [];
    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME);
    const records = (await this.#request(store.index(DOCUMENT_ID_INDEX).getAll(documentId))) as DeltaRecord[];
    return records.toSorted((a, b) => a.createdAt - b.createdAt);
  }

  async put(record: DeltaRecord): Promise<void> {
    if (this.#destroyed) return;
    const db = await this.#ensureDb();
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    await this.#request(transaction.objectStore(STORE_NAME).put(record));
    await this.#transaction(transaction);
  }

  async deleteMany(documentId: string, ids: string[]): Promise<void> {
    if (this.#destroyed || ids.length === 0) return;
    const db = await this.#ensureDb();
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    await Promise.all(ids.map((id) => this.#request(store.delete([documentId, id])))); // 복합키로 삭제
    await this.#transaction(transaction);
  }

  destroy(): void {
    this.#destroyed = true;
    this.#db?.close();
    this.#db = null;
  }
}

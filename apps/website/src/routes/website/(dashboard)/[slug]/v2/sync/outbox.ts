const DB_NAME = 'typie:outbox';
const DB_VERSION = 1;
const STORE_NAME = 'entries';

type StoredEntry = {
  id: string;
  documentId: string;
  bundle: Uint8Array;
  createdAt: number;
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
        store.createIndex('documentId', 'documentId');
      }
    });
  });
}

function idbRequest<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.addEventListener('error', () => reject(request.error));
    request.addEventListener('success', () => resolve(request.result));
  });
}

export class Outbox {
  #documentId: string;
  #db: IDBDatabase | null = null;

  constructor(documentId: string) {
    this.#documentId = documentId;
  }

  async #ensureDb(): Promise<IDBDatabase> {
    return (this.#db ??= await openDatabase());
  }

  async save(id: string, bundle: Uint8Array): Promise<void> {
    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME);
    await idbRequest(store.put({ id, documentId: this.#documentId, bundle, createdAt: Date.now() } satisfies StoredEntry));
  }

  async delete(id: string): Promise<void> {
    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME);
    await idbRequest(store.delete(id));
  }

  async loadAll(): Promise<{ id: string; bundle: Uint8Array }[]> {
    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME);
    const index = store.index('documentId');
    const results = (await idbRequest(index.getAll(this.#documentId))) as StoredEntry[];
    return results.toSorted((a, b) => a.createdAt - b.createdAt).map(({ id, bundle }) => ({ id, bundle }));
  }

  destroy(): void {
    this.#db?.close();
    this.#db = null;
  }
}

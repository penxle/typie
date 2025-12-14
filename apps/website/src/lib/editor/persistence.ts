const DB_NAME = 'typie-editor';
const DB_VERSION = 1;
const STORE_NAME = 'documents';

type StoredDocument = {
  id: string;
  snapshot: Uint8Array | null;
  pendingUpdates: Uint8Array[];
  updatedAt: number;
};

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.addEventListener('error', () => reject(request.error));
    request.addEventListener('success', () => resolve(request.result));

    request.addEventListener('upgradeneeded', () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'id' });
      }
    });
  });
}

export class IndexeddbPersistence {
  #documentId: string;
  #db: IDBDatabase | null = null;
  #destroyed = false;

  constructor(documentId: string) {
    this.#documentId = documentId;
  }

  async #ensureDb(): Promise<IDBDatabase> {
    if (this.#db) return this.#db;
    this.#db = await openDatabase();
    return this.#db;
  }

  async load(): Promise<{ snapshot: Uint8Array | null; pendingUpdates: Uint8Array[] } | null> {
    if (this.#destroyed) return null;

    const db = await this.#ensureDb();

    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, 'readonly');
      const store = transaction.objectStore(STORE_NAME);
      const request = store.get(this.#documentId);

      request.addEventListener('error', () => reject(request.error));
      request.addEventListener('success', () => {
        const result = request.result as StoredDocument | undefined;
        if (result) {
          resolve({ snapshot: result.snapshot, pendingUpdates: result.pendingUpdates });
        } else {
          resolve(null);
        }
      });
    });
  }

  async storeUpdate(update: Uint8Array): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();

    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, 'readwrite');
      const store = transaction.objectStore(STORE_NAME);
      const getRequest = store.get(this.#documentId);

      getRequest.addEventListener('error', () => reject(getRequest.error));
      getRequest.addEventListener('success', () => {
        const existing = getRequest.result as StoredDocument | undefined;
        const data: StoredDocument = {
          id: this.#documentId,
          snapshot: existing?.snapshot ?? null,
          pendingUpdates: [...(existing?.pendingUpdates ?? []), update],
          updatedAt: Date.now(),
        };
        const putRequest = store.put(data);

        putRequest.addEventListener('error', () => reject(putRequest.error));
        putRequest.addEventListener('success', () => resolve());
      });
    });
  }

  async saveSnapshot(snapshot: Uint8Array): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();

    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, 'readwrite');
      const store = transaction.objectStore(STORE_NAME);
      const data: StoredDocument = {
        id: this.#documentId,
        snapshot,
        pendingUpdates: [],
        updatedAt: Date.now(),
      };
      const request = store.put(data);

      request.addEventListener('error', () => reject(request.error));
      request.addEventListener('success', () => resolve());
    });
  }

  async clear(): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();

    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, 'readwrite');
      const store = transaction.objectStore(STORE_NAME);
      const request = store.delete(this.#documentId);

      request.addEventListener('error', () => reject(request.error));
      request.addEventListener('success', () => resolve());
    });
  }

  destroy(): void {
    this.#destroyed = true;
    this.#db?.close();
    this.#db = null;
  }
}

const DB_NAME = 'typie:documents';
const DB_VERSION = 1;
const STORE_NAME = 'documents';

type StoredDocument = {
  id: string;
  snapshot: Uint8Array;
  updates: Uint8Array[];
  version: Uint8Array;
  checkpoint: Uint8Array;
  updatedAt: number;
  generation: number;
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
  #version: Uint8Array = new Uint8Array();
  #checkpoint: Uint8Array = new Uint8Array();
  #generation = 0;

  constructor(documentId: string) {
    this.#documentId = documentId;
  }

  get version(): Uint8Array {
    return this.#version;
  }

  get checkpoint(): Uint8Array {
    return this.#checkpoint;
  }

  get generation(): number {
    return this.#generation;
  }

  async #ensureDb(): Promise<IDBDatabase> {
    if (this.#db) return this.#db;
    let db = await openDatabase();

    if (!db.objectStoreNames.contains(STORE_NAME)) {
      db.close();
      await new Promise<void>((resolve, reject) => {
        const request = indexedDB.deleteDatabase(DB_NAME);
        request.addEventListener('success', () => resolve());
        request.addEventListener('error', () => reject(request.error));
      });
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

  async #getAndUpdate(updater: (existing: StoredDocument | undefined) => StoredDocument | null): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    const existing = (await this.#request(store.get(this.#documentId))) as StoredDocument | undefined;
    const updated = updater(existing);
    if (updated) {
      await this.#request(store.put(updated));
    }
  }

  async load(): Promise<{ snapshot: Uint8Array; updates: Uint8Array[] } | null> {
    if (this.#destroyed) return null;

    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME);
    const result = (await this.#request(store.get(this.#documentId))) as StoredDocument | undefined;

    if (!result) {
      return null;
    }

    this.#version = result.version;
    this.#checkpoint = result.checkpoint;
    this.#generation = result.generation ?? 0;

    return {
      snapshot: result.snapshot,
      updates: result.updates,
    };
  }

  async saveUpdate(update: Uint8Array): Promise<void> {
    return this.#getAndUpdate((existing) => {
      if (!existing) return null;
      return {
        ...existing,
        updates: [...(existing.updates ?? []), update],
        updatedAt: Date.now(),
      };
    });
  }

  async saveSnapshot(snapshot: Uint8Array, version: Uint8Array, generation?: number): Promise<void> {
    if (this.#destroyed) return;

    if (generation !== undefined) {
      this.#generation = generation;
    }

    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME);
    await this.#request(
      store.put({
        id: this.#documentId,
        snapshot,
        updates: [],
        version,
        checkpoint: this.#checkpoint,
        updatedAt: Date.now(),
        generation: this.#generation,
      }),
    );
    this.#version = version;
  }

  async saveCheckpoint(checkpoint: Uint8Array): Promise<void> {
    this.#checkpoint = checkpoint;
    return this.#getAndUpdate((existing) => {
      if (!existing) return null;
      return { ...existing, checkpoint, updatedAt: Date.now() };
    });
  }

  async clear(): Promise<void> {
    if (this.#destroyed) return;

    const db = await this.#ensureDb();
    const store = db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME);
    await this.#request(store.delete(this.#documentId));
    this.#version = new Uint8Array();
    this.#checkpoint = new Uint8Array();
    this.#generation = 0;
  }

  destroy(): void {
    this.#destroyed = true;
    this.#db?.close();
    this.#db = null;
  }
}

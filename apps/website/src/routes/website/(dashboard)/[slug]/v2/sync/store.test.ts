import 'fake-indexeddb/auto';

import { afterEach, describe, expect, it } from 'vitest';
import { IndexeddbDeltaStore } from './store';

const bytes = (n: number) => new Uint8Array([n, n + 1, n + 2]);

describe('IndexeddbDeltaStore', () => {
  let store: IndexeddbDeltaStore | null = null;
  afterEach(() => {
    store?.destroy();
    store = null;
    indexedDB.deleteDatabase('typie:changeset-delta');
  });

  it('put then load returns records for the document, sorted by createdAt', async () => {
    store = new IndexeddbDeltaStore();
    await store.put({ id: 'a', documentId: 'doc1', changeset: bytes(1), createdAt: 2 });
    await store.put({ id: 'b', documentId: 'doc1', changeset: bytes(4), createdAt: 1 });
    await store.put({ id: 'c', documentId: 'doc2', changeset: bytes(7), createdAt: 1 });

    const docs = await store.load('doc1');
    expect(docs.map((r) => r.id)).toEqual(['b', 'a']);
  });

  it('put with same id is idempotent (upsert)', async () => {
    store = new IndexeddbDeltaStore();
    await store.put({ id: 'x', documentId: 'doc1', changeset: bytes(1), createdAt: 1 });
    await store.put({ id: 'x', documentId: 'doc1', changeset: bytes(1), createdAt: 1 });
    const docs = await store.load('doc1');
    expect(docs).toHaveLength(1);
  });

  it('REGRESSION(round-4 CRIT): same id in different documents does NOT collide', async () => {
    store = new IndexeddbDeltaStore();
    // 결정적 seed dot '1:0'이 두 새 문서에서 충돌하면 안 됨.
    await store.put({ id: '1:0', documentId: 'docA', changeset: bytes(1), createdAt: 1 });
    await store.put({ id: '1:0', documentId: 'docB', changeset: bytes(9), createdAt: 1 });
    const recsA = await store.load('docA');
    const recsB = await store.load('docB');
    expect(recsA.map((r) => [...r.changeset])).toEqual([[1, 2, 3]]);
    expect(recsB.map((r) => [...r.changeset])).toEqual([[9, 10, 11]]);
  });

  it('deleteMany removes only the given ids, scoped by document', async () => {
    store = new IndexeddbDeltaStore();
    await store.put({ id: '99:0', documentId: 'doc1', changeset: bytes(1), createdAt: 1 });
    await store.put({ id: '99:1', documentId: 'doc1', changeset: bytes(2), createdAt: 2 });
    await store.put({ id: '99:0', documentId: 'doc2', changeset: bytes(5), createdAt: 1 });
    await store.deleteMany('doc1', ['99:0']);
    const doc1Recs = await store.load('doc1');
    const doc2Recs = await store.load('doc2');
    expect(doc1Recs.map((r) => r.id)).toEqual(['99:1']);
    expect(doc2Recs.map((r) => r.id)).toEqual(['99:0']); // 다른 문서 영향 없음
  });
});

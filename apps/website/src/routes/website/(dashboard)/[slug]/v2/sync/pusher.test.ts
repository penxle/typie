import { describe, expect, it } from 'vitest';
import { Pusher } from './pusher.svelte';
import { dec, enc, FakeEditor, FakeStore } from './test-fakes';
import type { PusherOpts } from './types';

const baseOpts = (editor: FakeEditor, store: FakeStore, pushFn: PusherOpts['pushFn']) => ({
  editor,
  documentId: 'doc1',
  initialServerHeads: enc(),
  initialDurableHeads: enc(),
  store,
  pushFn,
});

describe('Pusher (single-source-of-truth)', () => {
  it('REGRESSION(orig bug): stale store delta is adopted and pushed without throwing', async () => {
    const editor = new FakeEditor([]);
    const store = new FakeStore();
    store.records.push(
      { id: '7', documentId: 'doc1', changeset: enc(7), createdAt: 1 },
      { id: '8', documentId: 'doc1', changeset: enc(8), createdAt: 2 },
    );
    const pushed: Uint8Array[] = [];
    const pusher = new Pusher(
      baseOpts(editor, store, async (cs) => {
        pushed.push(cs);
        editor.receiveRemoteChangeset(cs);
        const h = editor.currentHeads();
        return { heads: h, durableHeads: h };
      }),
    );
    await pusher.flushNow();
    expect(pushed.flatMap(dec).toSorted((a, b) => a - b)).toEqual([7, 8]);
  });

  it('REGRESSION(round-1 CRIT #1): durableHeads with an unknown concurrent dot never throws', async () => {
    const editor = new FakeEditor([1, 2]);
    const store = new FakeStore();
    const pusher = new Pusher(baseOpts(editor, store, async () => ({ heads: enc(2), durableHeads: enc(2) })));
    pusher.setDurableHeads(enc(2, 999));
    await expect(pusher.flushNow()).resolves.toBeUndefined();
  });

  it('REGRESSION(round-2 CRIT, durability): a record is NOT pruned until its op is in durableHeads', async () => {
    const editor = new FakeEditor([5]);
    const store = new FakeStore();
    const pusher = new Pusher({
      editor,
      documentId: 'doc1',
      initialServerHeads: enc(),
      initialDurableHeads: enc(),
      store,
      pushFn: async () => ({ heads: enc(5), durableHeads: enc() }),
    });
    await pusher.flushNow();
    await new Promise((r) => setTimeout(r, 0));
    const recsBeforeDurable = await store.load('doc1');
    expect(recsBeforeDurable.map((r) => r.id)).toEqual(['5']);
    pusher.setDurableHeads(enc(5));
    await new Promise((r) => setTimeout(r, 0));
    const recsAfterDurable = await store.load('doc1');
    expect(recsAfterDurable.length).toBe(0);
  });

  it('REGRESSION(round-3 CRIT): an acked op is not re-pushed every cycle (no resend-until-durable)', async () => {
    const editor = new FakeEditor([5]);
    const store = new FakeStore();
    let pushes = 0;
    const pusher = new Pusher({
      editor,
      documentId: 'doc1',
      initialServerHeads: enc(),
      initialDurableHeads: enc(),
      store,
      pushFn: async () => {
        pushes++;
        return { heads: enc(5), durableHeads: enc() };
      },
    });
    await pusher.flushNow();
    await pusher.flushNow();
    await pusher.flushNow();
    expect(pushes).toBe(1);
  });

  it('capture appends new changeset by first-op dot id and advances capturedHeads', async () => {
    const editor = new FakeEditor([1, 2]);
    const store = new FakeStore();
    const pusher = new Pusher({
      editor,
      documentId: 'doc1',
      initialServerHeads: enc(2),
      initialDurableHeads: enc(2),
      store,
      pushFn: async () => ({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() }),
    });
    await new Promise((r) => setTimeout(r, 0));
    editor.known.add(3);
    await pusher.captureNow();
    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id)).toEqual(['3']);
  });

  it('prune deletes only records proven server-durable-and-local', async () => {
    const editor = new FakeEditor([1, 2, 3]);
    const store = new FakeStore();
    const pusher = new Pusher(baseOpts(editor, store, async () => ({ heads: enc(3), durableHeads: enc(3) })));
    await pusher.flushNow();
    await new Promise((r) => setTimeout(r, 0));
    const pruneRecs = await store.load('doc1');
    expect(pruneRecs.length).toBe(0);
  });

  it('round-1 CRIT #3: a tab WITHOUT the op in its graph never prunes a sibling crash-durable record', async () => {
    const editor = new FakeEditor([]);
    const store = new FakeStore();
    store.records.push({ id: '30', documentId: 'doc1', changeset: enc(30), createdAt: 1 });
    const pusher = new Pusher(baseOpts(editor, store, async () => ({ heads: enc(), durableHeads: enc() })));
    pusher.setDurableHeads(enc());
    await new Promise((r) => setTimeout(r, 0));
    const crashRecs = await store.load('doc1');
    expect(crashRecs.map((r) => r.id)).toEqual(['30']);
  });
});

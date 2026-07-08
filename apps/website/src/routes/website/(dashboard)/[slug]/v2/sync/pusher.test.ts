import { describe, expect, it } from 'vitest';
import { Pusher } from './pusher.svelte';
import { dec, enc, FakeEditor, FakeStore } from './test-fakes';
import type { PusherEvent, PusherOpts } from './types';

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

  it('schedule() persists new local changesets immediately, decoupled from the push cadence', async () => {
    const editor = new FakeEditor([]);
    const store = new FakeStore();
    let pushes = 0;
    const pusher = new Pusher(
      baseOpts(editor, store, async () => {
        pushes++;
        return { heads: editor.currentHeads(), durableHeads: enc() };
      }),
    );
    await new Promise((r) => setTimeout(r, 0));
    const pushesAfterInit = pushes;

    editor.known.add(9);
    pusher.schedule();
    await new Promise((r) => setTimeout(r, 0));

    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id)).toEqual(['9']);
    expect(pushes).toBe(pushesAfterInit); // push waits for the idle window; durability must not
    pusher.stop();
  });

  it('REGRESSION: an op applied while a persist write is in-flight is not swallowed by capturedHeads', async () => {
    const editor = new FakeEditor([]);
    const store = new FakeStore();
    let injected = false;
    const origPut = store.put.bind(store);
    store.put = async (r) => {
      await origPut(r);
      if (!injected) {
        injected = true;
        editor.known.add(11); // edit lands while the previous record's write is awaited
      }
    };
    const pusher = new Pusher(baseOpts(editor, store, async () => ({ heads: editor.currentHeads(), durableHeads: enc() })));
    await new Promise((r) => setTimeout(r, 0));

    editor.known.add(10);
    pusher.schedule();
    await new Promise((r) => setTimeout(r, 0));
    pusher.schedule();
    await new Promise((r) => setTimeout(r, 0));

    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id).toSorted((a, b) => Number(a) - Number(b))).toEqual(['10', '11']);
    pusher.stop();
  });

  it('a failing store write does not kill later persists; the failed delta is retried', async () => {
    const editor = new FakeEditor([]);
    const store = new FakeStore();
    let failOnce = true;
    const origPut = store.put.bind(store);
    store.put = async (r) => {
      if (failOnce) {
        failOnce = false;
        throw new Error('quota');
      }
      await origPut(r);
    };
    const pusher = new Pusher(baseOpts(editor, store, async () => ({ heads: editor.currentHeads(), durableHeads: enc() })));
    await new Promise((r) => setTimeout(r, 0));

    editor.known.add(10);
    pusher.schedule();
    await new Promise((r) => setTimeout(r, 0));
    pusher.schedule();
    await new Promise((r) => setTimeout(r, 0));

    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id)).toEqual(['10']);
    pusher.stop();
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

  it('a persistently failing capture does not block the push of already-sealed changesets', async () => {
    const editor = new FakeEditor([5]);
    const store = new FakeStore();
    store.put = async () => {
      throw new Error('quota');
    };
    const pushed: Uint8Array[] = [];
    const pusher = new Pusher(
      baseOpts(editor, store, async (cs) => {
        pushed.push(cs);
        return { heads: enc(5), durableHeads: enc(5) };
      }),
    );
    await new Promise((r) => setTimeout(r, 0));
    expect(pushed.flatMap(dec)).toEqual([5]);
    expect(pusher.status).toBe('retrying');
    pusher.stop();
  });

  it('repeated capture failures are observable and reset once capture recovers', async () => {
    const editor = new FakeEditor([5]);
    const store = new FakeStore();
    let failing = true;
    const origPut = store.put.bind(store);
    store.put = async (r) => {
      if (failing) throw new Error('quota');
      await origPut(r);
    };
    let pushes = 0;
    const pusher = new Pusher(
      baseOpts(editor, store, async () => {
        pushes++;
        return { heads: enc(5), durableHeads: enc() };
      }),
    );
    await new Promise((r) => setTimeout(r, 0));
    expect(pusher.captureFailures).toBe(1);
    expect(pusher.status).toBe('retrying');

    window.dispatchEvent(new Event('online'));
    await new Promise((r) => setTimeout(r, 0));
    expect(pusher.captureFailures).toBe(2);
    expect(pusher.status).toBe('retrying');

    failing = false;
    window.dispatchEvent(new Event('online'));
    await new Promise((r) => setTimeout(r, 0));
    expect(pusher.captureFailures).toBe(0);
    expect(pusher.status).toBe('idle');
    expect(pushes).toBe(1);
    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id)).toEqual(['5']);
    pusher.stop();
  });

  it('withheld > 0 does not advance capturedHeads: the same window is re-requested and only emitted bytes are stored', async () => {
    const editor = new FakeEditor([1, 2]);
    const store = new FakeStore();
    const pusher = new Pusher({
      editor,
      documentId: 'doc1',
      initialServerHeads: enc(2),
      initialDurableHeads: enc(2),
      store,
      pushFn: async () => ({ heads: editor.currentHeads(), durableHeads: enc(2) }),
    });
    await new Promise((r) => setTimeout(r, 0));

    editor.known.add(3);
    editor.known.add(4);
    editor.withheld = 1;
    editor.missingCalls.length = 0;
    await pusher.captureNow();
    await pusher.captureNow();

    expect(editor.missingCalls).toEqual([[2], [2]]);
    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id)).toEqual(['3']);
    pusher.stop();
  });

  it('withheld > 0 skips prune for the cycle: no record is deleted from the store', async () => {
    const editor = new FakeEditor([5]);
    const store = new FakeStore();
    let deleteCalls = 0;
    const origDeleteMany = store.deleteMany.bind(store);
    store.deleteMany = async (documentId, ids) => {
      deleteCalls++;
      await origDeleteMany(documentId, ids);
    };
    const pusher = new Pusher(baseOpts(editor, store, async () => ({ heads: enc(5), durableHeads: enc() })));
    await pusher.flushNow();
    await new Promise((r) => setTimeout(r, 0));
    const deleteCallsBefore = deleteCalls;

    editor.withheld = 1;
    pusher.setDurableHeads(enc(5));
    await new Promise((r) => setTimeout(r, 0));

    expect(deleteCalls).toBe(deleteCallsBefore);
    const recs = await store.load('doc1');
    expect(recs.map((r) => r.id)).toEqual(['5']);
    pusher.stop();
  });

  it('withheld > 0 still pushes the emitted prefix and surfaces the withheld signal', async () => {
    const editor = new FakeEditor([1, 2]);
    const store = new FakeStore();
    const events: PusherEvent[] = [];
    const pushed: Uint8Array[] = [];
    const pusher = new Pusher({
      editor,
      documentId: 'doc1',
      initialServerHeads: enc(2),
      initialDurableHeads: enc(2),
      store,
      pushFn: async (cs) => {
        pushed.push(cs);
        return { heads: enc(2), durableHeads: enc(2) };
      },
      onEvent: (e) => {
        events.push(e);
      },
    });
    await new Promise((r) => setTimeout(r, 0));

    editor.known.add(3);
    editor.known.add(4);
    editor.withheld = 1;
    await pusher.flushNow();

    expect(pushed.flatMap(dec)).toEqual([3]);
    expect(events.some((e) => e.kind === 'persist.withheld' && e.count === 1)).toBe(true);
    pusher.stop();
  });
});

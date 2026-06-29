import { expect, it } from 'vitest';
import { Pusher } from './pusher.svelte';
import { dec, enc, FakeEditor, FakeStore } from './test-fakes';

it('two tabs converge: each tab pushes its own ops; server dedups; both end with all ops', async () => {
  const store = new FakeStore();
  const server = new Set<number>();
  const serverHeads = () => enc(server.size > 0 ? Math.max(...server) : 0);
  const pushFn = (cs: Uint8Array) => {
    for (const id of dec(cs)) server.add(id);
    const h = serverHeads();
    return Promise.resolve({ heads: h, durableHeads: h });
  };

  const tabA = new FakeEditor([]);
  const tabB = new FakeEditor([]);
  const pa = new Pusher({ editor: tabA, documentId: 'd', initialServerHeads: enc(0), initialDurableHeads: enc(0), store, pushFn });
  const pb = new Pusher({ editor: tabB, documentId: 'd', initialServerHeads: enc(0), initialDurableHeads: enc(0), store, pushFn });

  tabA.known.add(10);
  await pa.flushNow();
  tabB.known.add(20);
  await pb.flushNow();
  expect([...server].toSorted((a, b) => a - b)).toEqual([10, 20]);

  tabA.receiveRemoteChangeset(enc(20));
  pa.setDurableHeads(serverHeads());
  tabB.receiveRemoteChangeset(enc(10));
  pb.setDurableHeads(serverHeads());
  expect([...tabA.known].toSorted((a, b) => a - b)).toEqual([10, 20]);
  expect([...tabB.known].toSorted((a, b) => a - b)).toEqual([10, 20]);

  pa.stop();
  pb.stop();
});

it('crash durability: tab A persists then "crashes" before push; tab B pushes A ops from shared store', async () => {
  const store = new FakeStore();
  const server = new Set<number>();
  const pushFn = (cs: Uint8Array) => {
    for (const id of dec(cs)) server.add(id);
    const h = enc(server.size > 0 ? Math.max(...server) : 0);
    return Promise.resolve({ heads: h, durableHeads: h });
  };

  const tabA = new FakeEditor([]);
  const pa = new Pusher({
    editor: tabA,
    documentId: 'd',
    initialServerHeads: enc(),
    initialDurableHeads: enc(),
    store,
    pushFn: () => Promise.reject(new Error('offline')),
  });
  tabA.known.add(30);
  await pa.captureNow();
  pa.stop();
  const storeRecs = await store.load('d');
  expect(storeRecs.map((r) => r.id)).toEqual(['30']);

  const tabB = new FakeEditor([]);
  const pb = new Pusher({ editor: tabB, documentId: 'd', initialServerHeads: enc(), initialDurableHeads: enc(), store, pushFn });
  await pb.flushNow();
  await new Promise((r) => setTimeout(r, 0));
  expect([...server]).toEqual([30]);
  expect([...tabB.known]).toEqual([30]);

  pb.stop();
});

import assert from 'node:assert/strict';
import { once } from 'node:events';
import test from 'node:test';
import { serve, upgradeWebSocket } from '@hono/node-server';
import { Hono } from 'hono';
import { WebSocket, WebSocketServer } from 'ws';
import { decodeRaw, encodeMessage, SUBPROTOCOL } from './protocol.ts';
import { createSyncServer } from './server.ts';
import { FakeSyncDeps } from './testing.ts';
import type { AddressInfo } from 'node:net';
import type { ClientMessage, ServerMessage } from './protocol.ts';

const startServer = async (deps: FakeSyncDeps, checkMaintenance: () => Promise<boolean> = async () => false) => {
  const app = new Hono();
  app.get(
    '/hono-ws',
    upgradeWebSocket(() => ({})),
  );
  const wss = new WebSocketServer({ noServer: true });
  const server = serve({ fetch: app.fetch, hostname: '127.0.0.1', port: 0, websocket: { server: wss } });
  await once(server, 'listening');
  const sync = createSyncServer({ deps, checkMaintenance });
  server.on('upgrade', (request, socket, head) => {
    if (!sync.shouldHandle(request)) return;
    sync.handleUpgrade(request, socket, head);
  });
  return server;
};

const stopServer = (server: ReturnType<typeof serve>) => new Promise<void>((resolve) => void server.close(() => resolve()));

class Client {
  #waiters: (() => void)[] = [];
  ws: WebSocket;
  received: ServerMessage[] = [];

  constructor(port: number) {
    this.ws = new WebSocket(`ws://127.0.0.1:${port}/sync`, SUBPROTOCOL);
    this.ws.on('message', (data: Buffer) => {
      this.received.push(decodeRaw(new Uint8Array(data)) as ServerMessage);
      // eslint-disable-next-line unicorn/no-unnecessary-splice -- drain: splice return is iterated
      for (const waiter of this.#waiters.splice(0)) waiter();
    });
  }

  send(message: ClientMessage): void {
    this.ws.send(encodeMessage(message));
  }

  async waitFor<T extends ServerMessage['t']>(t: T, timeoutMs = 5000): Promise<ServerMessage & { t: T }> {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
      const found = this.received.find((m) => m.t === t);
      if (found) return found as ServerMessage & { t: T };
      if (Date.now() > deadline) throw new Error(`timeout waiting for ${t}`);
      await new Promise<void>((resolve) => {
        const timer = setTimeout(resolve, 100);
        this.#waiters.push(() => {
          clearTimeout(timer);
          resolve();
        });
      });
    }
  }
}

test('E2E: hello → attach → snapshot → tail → live → push → echo 억제', async () => {
  const deps = new FakeSyncDeps();
  deps.tickets.set('TK-A', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });
  deps.tickets.set('TK-B', { sessionId: 'S2', userId: 'U2', deviceId: 'DEV2' });
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1, 2, 3) }]);
  deps.collectedSeq.set('D1', '1-0');
  deps.seedStream('D1', [{ seq: '2-0', changeset: Uint8Array.of(4) }]);
  deps.liveHeads.set('D1', Uint8Array.of(0xaa));
  deps.durableHeadsMap.set('D1', Uint8Array.of(0xdd));

  const server = await startServer(deps);
  const port = (server.address() as AddressInfo).port;

  const a = new Client(port);
  await once(a.ws, 'open');
  a.send({ t: 'hello', ticket: 'TK-A', clientId: 'client-a', capabilities: [] });
  await a.waitFor('hello-ack');

  a.send({ t: 'attach', documentId: 'D1' });
  const chunk = await a.waitFor('snapshot-chunk');
  assert.equal(chunk.rowId, 'B1');
  assert.deepEqual(new Uint8Array(chunk.bytes), Uint8Array.of(1, 2, 3));
  const end = await a.waitFor('snapshot-end');
  assert.equal(end.seq, '1-0');
  const tail = await a.waitFor('changesets');
  assert.equal(tail.seq, '2-0');

  const b = new Client(port);
  await once(b.ws, 'open');
  b.send({ t: 'hello', ticket: 'TK-B', clientId: 'client-b', capabilities: [] });
  await b.waitFor('hello-ack');
  b.send({ t: 'push', id: 'p1', documentId: 'D1', changesets: Uint8Array.of(9, 9) });
  const ack = await b.waitFor('push-ack');
  assert.ok(ack.heads.length > 0);

  const live = await (async () => {
    for (let i = 0; i < 500; i += 1) {
      const found = a.received.filter((m) => m.t === 'changesets').find((m) => m.t === 'changesets' && m.seq !== '2-0');
      if (found) return found;
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    throw new Error('timeout waiting for live changesets');
  })();
  if (live.t !== 'changesets') return assert.fail();
  assert.deepEqual(
    live.bundles.map((x) => new Uint8Array(x)),
    [Uint8Array.of(9, 9)],
  );

  a.send({ t: 'push', id: 'p2', documentId: 'D1', changesets: Uint8Array.of(5, 5) });
  await a.waitFor('push-ack');
  await new Promise((resolve) => setTimeout(resolve, 100));
  const echoed = a.received
    .filter((m) => m.t === 'changesets')
    .some((m) => m.t === 'changesets' && m.bundles.some((x) => new Uint8Array(x)[0] === 5));
  assert.equal(echoed, false);

  const bChangesets = b.received.filter((m) => m.t === 'changesets');
  assert.equal(bChangesets.length, 0);

  a.ws.close();
  b.ws.close();
  await stopServer(server);
});

test('E2E: 발신자도 attach 상태면 자기 push에 대해 빈 changesets 프레임(seq/heads)을 받는다(본문 제외)', async () => {
  const deps = new FakeSyncDeps();
  deps.tickets.set('TK', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });
  deps.liveHeads.set('D1', Uint8Array.of(0xaa));
  deps.durableHeadsMap.set('D1', Uint8Array.of(0xdd));

  const server = await startServer(deps);
  const port = (server.address() as AddressInfo).port;

  const client = new Client(port);
  try {
    await once(client.ws, 'open');
    client.send({ t: 'hello', ticket: 'TK', clientId: 'client-a', capabilities: [] });
    await client.waitFor('hello-ack');

    client.send({ t: 'attach', documentId: 'D1' });
    await client.waitFor('snapshot-end');

    client.send({ t: 'push', id: 'p1', documentId: 'D1', changesets: Uint8Array.of(7, 7) });
    const ack = await client.waitFor('push-ack');

    const notify = await (async () => {
      for (let i = 0; i < 200; i += 1) {
        const found = client.received.filter((m) => m.t === 'changesets').find((m) => m.t === 'changesets' && m.bundles.length === 0);
        if (found) return found;
        await new Promise((resolve) => setTimeout(resolve, 10));
      }
      throw new Error('timeout waiting for sender seq-only notify');
    })();
    if (notify.t !== 'changesets') return assert.fail();
    assert.equal(notify.seq, '1-0');
    assert.deepEqual(notify.heads, ack.heads);
    assert.deepEqual(notify.durableHeads, ack.durableHeads);

    const bodyDelivered = client.received
      .filter((m) => m.t === 'changesets')
      .some((m) => m.t === 'changesets' && m.bundles.some((b) => new Uint8Array(b)[0] === 7));
    assert.equal(bodyDelivered, false);
  } finally {
    client.ws.close();
    await stopServer(server);
  }
});

test('E2E: 잘못된 티켓 → close 4001', async () => {
  const deps = new FakeSyncDeps();
  const server = await startServer(deps);
  const port = (server.address() as AddressInfo).port;
  const client = new Client(port);
  await once(client.ws, 'open');
  client.send({ t: 'hello', ticket: 'NOPE', clientId: 'c', capabilities: [] });
  const [code] = (await once(client.ws, 'close')) as [number];
  assert.equal(code, 4001);
  await stopServer(server);
});

test('E2E: 재개 — snapshotCursor로 mid-row 이어받기', async () => {
  const deps = new FakeSyncDeps();
  deps.tickets.set('TK', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1, 2, 3, 4, 5, 6) }]);
  const server = await startServer(deps);
  const port = (server.address() as AddressInfo).port;
  const client = new Client(port);
  await once(client.ws, 'open');
  client.send({ t: 'hello', ticket: 'TK', clientId: 'c', capabilities: [] });
  await client.waitFor('hello-ack');
  client.send({ t: 'attach', documentId: 'D1', snapshotCursor: { rowId: 'B1', seq: 1, offset: 4 } });
  const chunk = await client.waitFor('snapshot-chunk');
  assert.equal(chunk.offset, 4);
  assert.deepEqual(new Uint8Array(chunk.bytes), Uint8Array.of(5, 6));
  await client.waitFor('snapshot-end');
  client.ws.close();
  await stopServer(server);
});

test('E2E: subprotocol 누락 → close 4003', async () => {
  const deps = new FakeSyncDeps();
  const server = await startServer(deps);
  const port = (server.address() as AddressInfo).port;
  const ws = new WebSocket(`ws://127.0.0.1:${port}/sync`);
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- ignore socket error in this test
  ws.on('error', () => {});
  const [code] = (await once(ws, 'close')) as [number];
  assert.equal(code, 4003);
  await stopServer(server);
});

test('E2E: 어댑터 공존 — hono upgradeWebSocket 라우트도 함께 동작', async () => {
  const deps = new FakeSyncDeps();
  const server = await startServer(deps);
  const port = (server.address() as AddressInfo).port;
  const ws = new WebSocket(`ws://127.0.0.1:${port}/hono-ws`);
  await once(ws, 'open');
  ws.close();
  await stopServer(server);
});

test('E2E: maintenance 중 /sync upgrade는 503 거부', async () => {
  const deps = new FakeSyncDeps();
  const server = await startServer(deps, async () => true);
  const port = (server.address() as AddressInfo).port;
  const ws = new WebSocket(`ws://127.0.0.1:${port}/sync`, SUBPROTOCOL);
  const message = await new Promise<string>((resolve) => {
    ws.on('error', (err) => resolve(err.message));
    ws.on('open', () => resolve('open'));
  });
  assert.match(message, /503/);
  await stopServer(server);
});

import assert from 'node:assert/strict';
import test from 'node:test';
import { MAX_BUFFERED_BYTES, PUSH_BUCKET_CAPACITY, SyncConnection } from './connection.ts';
import { decodeRaw, encodeMessage } from './protocol.ts';
import { FakeSyncDeps } from './testing.ts';
import type { SyncSocket } from './connection.ts';
import type { ClientMessage, ServerMessage } from './protocol.ts';

class FakeSocket implements SyncSocket {
  sent: ServerMessage[] = [];
  closed: { code: number; reason?: string } | null = null;
  buffered = 0;
  send = async (data: Uint8Array): Promise<void> => {
    this.sent.push(decodeRaw(data) as ServerMessage);
  };
  close = (code: number, reason?: string): void => {
    this.closed ??= { code, reason };
  };
  bufferedAmount = (): number => this.buffered;
}

const setup = (options: { now?: () => number; helloTimeoutMs?: number } = {}) => {
  const deps = new FakeSyncDeps();
  deps.tickets.set('TK', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });
  const socket = new FakeSocket();
  const connection = new SyncConnection({ deps, socket, now: options.now, helloTimeoutMs: options.helloTimeoutMs });
  const dispatch = (message: ClientMessage) => connection.handleMessage(encodeMessage(message));
  return { deps, socket, connection, dispatch };
};

const hello = async (d: ReturnType<typeof setup>) => {
  await d.dispatch({ t: 'hello', ticket: 'TK', clientId: 'me', capabilities: [] });
};

test('hello: 유효 티켓 → hello-ack, 티켓은 1회용', async () => {
  const d = setup();
  await hello(d);
  assert.deepEqual(d.socket.sent[0], { t: 'hello-ack', capabilities: [] });
  assert.equal(d.deps.tickets.has('TK'), false);
});

test('잘못된 티켓 → close 4001', async () => {
  const d = setup();
  await d.dispatch({ t: 'hello', ticket: 'BAD', clientId: 'me', capabilities: [] });
  assert.equal(d.socket.closed?.code, 4001);
});

test('hello 전 다른 메시지 → close 4003', async () => {
  const d = setup();
  await d.dispatch({ t: 'ping' });
  assert.equal(d.socket.closed?.code, 4003);
});

test('hello 타임아웃: 기한 내 hello가 없으면 close 4003', async () => {
  const d = setup({ helloTimeoutMs: 20 });
  await new Promise((resolve) => setTimeout(resolve, 60));
  assert.equal(d.socket.closed?.code, 4003);
});

test('hello 타임아웃: 기한 내 hello가 오면 유지된다', async () => {
  const d = setup({ helloTimeoutMs: 20 });
  await hello(d);
  await new Promise((resolve) => setTimeout(resolve, 60));
  assert.equal(d.socket.closed, null);
});

test('중복 hello → close 4003', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'hello', ticket: 'TK', clientId: 'me', capabilities: [] });
  assert.equal(d.socket.closed?.code, 4003);
});

test('ping → pong', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'ping' });
  assert.deepEqual(d.socket.sent.at(-1), { t: 'pong' });
});

test('attach: 권한 없으면 error(document, forbidden, permanent)', async () => {
  const d = setup();
  d.deps.access.set('D1', 'forbidden');
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  const error = d.socket.sent.at(-1);
  if (error?.t !== 'error') return assert.fail();
  assert.equal(error.scope, 'document');
  assert.equal(error.code, 'forbidden');
  assert.equal(error.permanent, true);
});

test('attach: v2 아니면 document_not_v2', async () => {
  const d = setup();
  d.deps.access.set('D1', 'not_v2');
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  const error = d.socket.sent.at(-1);
  if (error?.t !== 'error') return assert.fail();
  assert.equal(error.code, 'document_not_v2');
});

test('attach 성공: attach-ack와 snapshot-end 도달, 접근 체크는 연결당 1회 캐시', async () => {
  const d = setup();
  let checks = 0;
  const original = d.deps.checkDocumentAccess;
  d.deps.checkDocumentAccess = async (userId, documentId) => {
    checks += 1;
    return original(userId, documentId);
  };
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(d.socket.sent.some((m) => m.t === 'attach-ack'));
  assert.ok(d.socket.sent.some((m) => m.t === 'snapshot-end'));
  await d.dispatch({ t: 'push', id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1) });
  assert.equal(checks, 1);
});

test('sinceSeq와 snapshotCursor 동시 지정 → close 4003', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1', sinceSeq: '1-0', snapshotCursor: { rowId: 'B1', seq: 1, offset: 0 } });
  assert.equal(d.socket.closed?.code, 4003);
});

test('활성 채널 중복 attach → close 4003', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  assert.equal(d.socket.closed?.code, 4003);
});

test('detach 후 재attach 허용', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  await d.dispatch({ t: 'detach', documentId: 'D1' });
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(d.socket.closed, null);
});

test('push/pull이 requests 핸들러로 위임된다', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'push', id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1) });
  assert.ok(d.socket.sent.some((m) => m.t === 'push-ack'));
  await d.dispatch({ t: 'pull', id: 'r2', documentId: 'D1' });
  assert.ok(d.socket.sent.some((m) => m.t === 'pull-ack'));
});

test('push: 권한 없으면 error(request, forbidden)', async () => {
  const d = setup();
  d.deps.access.set('D1', 'forbidden');
  await hello(d);
  await d.dispatch({ t: 'push', id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1) });
  const error = d.socket.sent.at(-1);
  if (error?.t !== 'error') return assert.fail();
  assert.equal(error.scope, 'request');
  assert.equal(error.id, 'r1');
});

test('malformed 프레임 → close 4003, unknown 타입 → 무시', async () => {
  const d = setup();
  await hello(d);
  await d.connection.handleMessage(encodeMessage({ t: 'future-type' } as never));
  assert.equal(d.socket.closed === null, true);
  await d.connection.handleMessage(Uint8Array.of(0xff, 0x00));
  assert.equal(d.socket.closed?.code, 4003);
});

test('destroy: 모든 채널 stop, 이후 live 이벤트 미전달', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  d.connection.destroy();
  const before = d.socket.sent.length;
  d.deps.emit('D1', { target: '*', seq: '1-0', changesets: [], heads: '', durableHeads: '' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(d.socket.sent.length, before);
});

test('입력 직렬화: attach 처리 중 도착한 detach가 채널을 남기지 않는다', async () => {
  const d = setup();
  await hello(d);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const original = d.deps.checkDocumentAccess;
  d.deps.checkDocumentAccess = async (userId, documentId) => {
    await gate;
    return original(userId, documentId);
  };
  const attachDone = d.dispatch({ t: 'attach', documentId: 'D1' });
  const detachDone = d.dispatch({ t: 'detach', documentId: 'D1' });
  release();
  await attachDone;
  await detachDone;
  await new Promise((resolve) => setTimeout(resolve, 0));
  const before = d.socket.sent.length;
  d.deps.emit('D1', { target: '*', seq: '1-0', changesets: [], heads: '', durableHeads: '' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(d.socket.sent.length, before);
});

test('destroy 후 재개된 attach는 채널을 만들지 않는다', async () => {
  const d = setup();
  await hello(d);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const original = d.deps.checkDocumentAccess;
  d.deps.checkDocumentAccess = async (userId, documentId) => {
    await gate;
    return original(userId, documentId);
  };
  const attachDone = d.dispatch({ t: 'attach', documentId: 'D1' });
  d.connection.destroy();
  release();
  await attachDone;
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(
    d.socket.sent.some((m) => m.t === 'attach-ack'),
    false,
  );
});

test('hello 전 unknown 타입도 close 4003', async () => {
  const d = setup();
  await d.connection.handleMessage(encodeMessage({ t: 'future-type' } as never));
  assert.equal(d.socket.closed?.code, 4003);
});

test('자가 close 후 큐에 남은 메시지는 처리되지 않는다', async () => {
  const d = setup();
  await hello(d);
  const bad = d.connection.handleMessage(Uint8Array.of(0xff, 0x00));
  const late = d.connection.handleMessage(encodeMessage({ t: 'push', id: 'r9', documentId: 'D1', changesets: Uint8Array.of(1) }));
  await bad;
  await late;
  assert.equal(
    d.socket.sent.some((m) => m.t === 'push-ack'),
    false,
  );
});

test('bufferedAmount 초과 → close 4002', async () => {
  const d = setup();
  await hello(d);
  d.socket.buffered = MAX_BUFFERED_BYTES + 1;
  await d.dispatch({ t: 'ping' });
  assert.equal(d.socket.closed?.code, 4002);
});

test('push: 무구독이면 error(subscription_required, permanent) + append 없음', async () => {
  const d = setup();
  d.deps.writable = false;
  await hello(d);
  await d.dispatch({ t: 'push', id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1) });
  const error = d.socket.sent.at(-1);
  if (error?.t !== 'error') return assert.fail();
  assert.equal(error.scope, 'request');
  assert.equal(error.code, 'subscription_required');
  assert.equal(error.permanent, true);
  assert.equal((d.deps.stream.get('D1') ?? []).length, 0);
});

test('push: 구독 있으면 통과하고 checkWritable은 커넥션당 1회', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'push', id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1) });
  await d.dispatch({ t: 'push', id: 'r2', documentId: 'D1', changesets: Uint8Array.of(2) });
  assert.equal(d.deps.checkWritableCalls, 1);
  assert.equal(d.socket.sent.filter((m) => m.t === 'push-ack').length, 2);
});

test('pull: 무구독이어도 허용', async () => {
  const d = setup();
  d.deps.writable = false;
  await hello(d);
  await d.dispatch({ t: 'pull', id: 'r1', documentId: 'D1' });
  assert.notEqual(d.socket.sent.at(-1)?.t, 'error');
});

test('attach: presence lease를 연결 nonce로 mark (attach-ack 전 await)', async () => {
  const d = setup();
  await hello(d);
  let ackSeenAtMark: boolean | null = null;
  const original = d.deps.markPresence;
  d.deps.markPresence = async (documentId, connectionId) => {
    ackSeenAtMark = d.socket.sent.some((m) => m.t === 'attach-ack');
    return original(documentId, connectionId);
  };
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  assert.equal(d.deps.presenceMarks.length, 1);
  assert.equal(d.deps.presenceMarks[0].documentId, 'D1');
  assert.ok(d.deps.presenceMarks[0].connectionId.length > 0);
  assert.equal(ackSeenAtMark, false);
});

test('detach: 같은 연결 nonce로 presence clear', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  await d.dispatch({ t: 'detach', documentId: 'D1' });
  assert.equal(d.deps.presenceClears.length, 1);
  assert.equal(d.deps.presenceClears[0].documentId, 'D1');
  assert.equal(d.deps.presenceClears[0].connectionId, d.deps.presenceMarks[0].connectionId);
});

test('destroy: 첨부된 모든 문서 presence를 연결 nonce로 clear', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await d.dispatch({ t: 'attach', documentId: 'D2' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  const nonce = d.deps.presenceMarks[0].connectionId;
  d.connection.destroy();
  const cleared = d.deps.presenceClears.filter((c) => c.connectionId === nonce).map((c) => c.documentId);
  assert.deepEqual(new Set(cleared), new Set(['D1', 'D2']));
});

test('refreshPresence: 첨부 문서 lease를 같은 nonce로 갱신', async () => {
  const d = setup();
  await hello(d);
  await d.dispatch({ t: 'attach', documentId: 'D1' });
  await new Promise((resolve) => setTimeout(resolve, 0));
  const nonce = d.deps.presenceMarks[0].connectionId;
  d.connection.refreshPresence();
  const refreshed = d.deps.presenceMarks.filter((m) => m.documentId === 'D1' && m.connectionId === nonce);
  assert.ok(refreshed.length >= 2);
});

test('reconnect 경합: 구 연결 정리가 신 연결 lease를 지우지 않는다', async () => {
  const deps = new FakeSyncDeps();
  deps.tickets.set('TK1', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });
  deps.tickets.set('TK2', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });

  const socketA = new FakeSocket();
  const a = new SyncConnection({ deps, socket: socketA });
  await a.handleMessage(encodeMessage({ t: 'hello', ticket: 'TK1', clientId: 'me', capabilities: [] }));
  await a.handleMessage(encodeMessage({ t: 'attach', documentId: 'D1' }));

  const socketB = new FakeSocket();
  const b = new SyncConnection({ deps, socket: socketB });
  await b.handleMessage(encodeMessage({ t: 'hello', ticket: 'TK2', clientId: 'me', capabilities: [] }));
  await b.handleMessage(encodeMessage({ t: 'attach', documentId: 'D1' }));

  const nonceA = deps.presenceMarks[0].connectionId;
  const nonceB = deps.presenceMarks[1].connectionId;
  assert.notEqual(nonceA, nonceB);

  a.destroy();

  assert.ok(deps.presenceClears.some((c) => c.documentId === 'D1' && c.connectionId === nonceA));
  assert.equal(
    deps.presenceClears.some((c) => c.connectionId === nonceB),
    false,
  );
});

test('push rate limit: 용량 소진 시 rate_limited, 시간 경과로 refill', async () => {
  let clock = 0;
  const d = setup({ now: () => clock });
  await hello(d);
  for (let i = 0; i < PUSH_BUCKET_CAPACITY; i += 1) {
    await d.dispatch({ t: 'push', id: `r${i}`, documentId: 'D1', changesets: new Uint8Array() });
  }
  await d.dispatch({ t: 'push', id: 'over', documentId: 'D1', changesets: new Uint8Array() });
  const rejected = d.socket.sent.at(-1);
  if (rejected?.t !== 'error') return assert.fail();
  assert.equal(rejected.code, 'rate_limited');
  assert.equal(rejected.permanent, false);
  assert.equal(rejected.id, 'over');

  clock += 1000;
  await d.dispatch({ t: 'push', id: 'after', documentId: 'D1', changesets: new Uint8Array() });
  assert.equal(d.socket.sent.at(-1)?.t, 'push-ack');
});

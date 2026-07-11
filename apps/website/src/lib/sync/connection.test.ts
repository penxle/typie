/* eslint-disable unicorn/no-return-array-push -- SyncConnection#push collides with Array#push's rule heuristic */
import { describe, expect, test, vi } from 'vitest';
import { SyncConnection } from './connection';
import { FakeWebSocket } from './test-fakes';

const setup = () => {
  const sockets: FakeWebSocket[] = [];
  const connection = new SyncConnection({
    createSocket: () => {
      const socket = new FakeWebSocket();
      sockets.push(socket);
      return socket;
    },
    fetchTicket: vi.fn(async () => `TK-${sockets.length + 1}`),
  });
  return { connection, sockets };
};

const handshake = async (socket: FakeWebSocket) => {
  await vi.waitFor(() => expect(socket.onopen).not.toBeNull());
  socket.serverOpen();
  await vi.waitFor(() => expect(socket.lastOf('hello')).toBeDefined());
  socket.serverSend({ t: 'hello-ack', capabilities: [] });
};

describe('SyncConnection', () => {
  test('lazy connect: 첫 push가 티켓 발급→hello→push 순서를 만든다', async () => {
    const { connection, sockets } = setup();
    const pushPromise = connection.push('D1', Uint8Array.of(1));
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('push')).toBeDefined());
    const push = sockets[0].lastOf('push');
    expect(push?.documentId).toBe('D1');
    expect(sockets[0].lastOf('hello')?.ticket).toBe('TK-1');
    sockets[0].serverSend({ t: 'push-ack', id: push?.id, heads: Uint8Array.of(9), durableHeads: Uint8Array.of(8) });
    const result = await pushPromise;
    expect(new Uint8Array(result.heads)).toEqual(Uint8Array.of(9));
  });

  test('push 오류 응답은 SyncRequestError(permanent)로 reject', async () => {
    const { connection, sockets } = setup();
    const pushPromise = connection.push('D1', Uint8Array.of(1));
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('push')).toBeDefined());
    const id = sockets[0].lastOf('push')?.id;
    sockets[0].serverSend({ t: 'error', scope: 'request', id, code: 'invalid_changeset_payload', permanent: true });
    await expect(pushPromise).rejects.toMatchObject({ code: 'invalid_changeset_payload', permanent: true });
  });

  test('절단 시 대기 중 요청은 connection_lost(transient)로 reject되고 재연결에서 티켓을 재발급한다', async () => {
    const { connection, sockets } = setup();
    const pushPromise = connection.push('D1', Uint8Array.of(1));
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    sockets[0].serverClose(1006);
    await expect(pushPromise).rejects.toMatchObject({ code: 'connection_lost', permanent: false });
    const pushPromise2 = connection.push('D1', Uint8Array.of(2));
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 3000 });
    await handshake(sockets[1]);
    expect(sockets[1].lastOf('hello')?.ticket).toBe('TK-2');
    await vi.waitFor(() => expect(sockets[1].lastOf('push')).toBeDefined());
    const id = sockets[1].lastOf('push')?.id;
    sockets[1].serverSend({ t: 'push-ack', id, heads: new Uint8Array(), durableHeads: new Uint8Array() });
    await pushPromise2;
  });

  test('sendAttach는 ready 전에는 드랍되고 연결만 트리거한다 (재전송은 채널 소유)', async () => {
    const { connection, sockets } = setup();
    connection.sendAttach('D1', {});
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    expect(sockets[0].lastOf('attach')).toBeUndefined();
    connection.sendAttach('D1', {});
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
  });

  test('single-flight: 동시 요청 2건도 소켓·티켓은 1개', async () => {
    const { connection, sockets } = setup();
    const p1 = connection.push('D1', Uint8Array.of(1));
    const p2 = connection.push('D2', Uint8Array.of(2));
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'push').length).toBe(2));
    expect(sockets.length).toBe(1);
    const pushIds = sockets[0].sent.filter((m) => m.t === 'push').map((m) => (m.t === 'push' ? m.id : ''));
    for (const id of pushIds) {
      sockets[0].serverSend({ t: 'push-ack', id, heads: new Uint8Array(), durableHeads: new Uint8Array() });
    }
    await Promise.all([p1, p2]);
  });

  test('절단 시 reject된 요청은 재연결에서 유령 전송되지 않는다', async () => {
    const { connection, sockets } = setup();
    const pushPromise = connection.push('D1', Uint8Array.of(1));
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    sockets[0].serverClose(1006);
    await expect(pushPromise).rejects.toMatchObject({ code: 'connection_lost' });
    const pushPromise2 = connection.push('D1', Uint8Array.of(2));
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 5000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].sent.filter((m) => m.t === 'push').length).toBe(1));
    const id = sockets[1].lastOf('push')?.id;
    sockets[1].serverSend({ t: 'push-ack', id, heads: new Uint8Array(), durableHeads: new Uint8Array() });
    await pushPromise2;
  });

  test('스테일 소켓의 늦은 메시지는 무시된다 (소켓 바인딩)', async () => {
    const { connection, sockets } = setup();
    let reconnects = 0;
    connection.onReconnected(() => {
      reconnects += 1;
    });
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- no-op handler, message routing isn't under test here
    connection.registerChannel('D1', () => {});
    connection.sendAttach('D1', {});
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    const stale = sockets[0];
    stale.serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 5000 });
    const before = sockets[1].sent.length;
    const staleBefore = stale.sent.length;
    stale.serverSend({ t: 'hello-ack', capabilities: [] });
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(reconnects).toBe(1);
    expect(stale.sent.length).toBe(staleBefore);
    expect(sockets[1].sent.length).toBe(before);
  });

  test('registerChannel은 documentId별로 라우팅하고 해제된다', async () => {
    const { connection, sockets } = setup();
    const received: string[] = [];
    const off = connection.registerChannel('D1', (m) => {
      received.push(m.t);
    });
    connection.sendAttach('D1', {});
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    connection.sendAttach('D1', {});
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D2' });
    sockets[0].serverSend({ t: 'reload', documentId: 'D1' });
    await vi.waitFor(() => expect(received).toEqual(['attach-ack', 'reload']));
    off();
    sockets[0].serverSend({ t: 'reload', documentId: 'D1' });
    expect(received).toEqual(['attach-ack', 'reload']);
  });

  test('onReconnected는 hello-ack 재수립마다 호출된다', async () => {
    const { connection, sockets } = setup();
    let reconnects = 0;
    connection.onReconnected(() => {
      reconnects += 1;
    });
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- no-op handler, message routing isn't under test here
    connection.registerChannel('D1', () => {});
    connection.sendAttach('D1', {});
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(reconnects).toBe(1));
    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 3000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(reconnects).toBe(2));
  });

  test('ping: 서버 pong 2회 무응답 시 소켓을 닫고 재연결한다', async () => {
    vi.useFakeTimers();
    try {
      const { connection, sockets } = setup();
      connection.sendAttach('D1', {});
      await vi.waitFor(() => expect(sockets.length).toBe(1));
      sockets[0].serverOpen();
      await vi.waitFor(() => expect(sockets[0].lastOf('hello')).toBeDefined());
      sockets[0].serverSend({ t: 'hello-ack', capabilities: [] });
      await vi.advanceTimersByTimeAsync(30_000);
      expect(sockets[0].lastOf('ping')).toBeDefined();
      await vi.advanceTimersByTimeAsync(30_000);
      await vi.advanceTimersByTimeAsync(30_000);
      expect(sockets[0].closed).not.toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });
});

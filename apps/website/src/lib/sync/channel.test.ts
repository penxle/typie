/* eslint-disable unicorn/no-return-array-push -- subscriber() event log helpers return push() directly */
import { describe, expect, test, vi } from 'vitest';
import { DocumentChannels, loadDocumentSnapshot } from './channel';
import { SyncConnection } from './connection';
import { FakeWebSocket } from './test-fakes';
import type { ChannelSubscriber } from './channel';

const setup = () => {
  const sockets: FakeWebSocket[] = [];
  const connection = new SyncConnection({
    createSocket: () => {
      const socket = new FakeWebSocket();
      sockets.push(socket);
      return socket;
    },
    fetchTicket: async () => 'TK',
    pingIntervalMs: 120_000,
  });
  const channels = new DocumentChannels(connection, 10);
  return { channels, connection, sockets };
};

const handshake = async (socket: FakeWebSocket) => {
  await vi.waitFor(() => expect(socket.onopen).not.toBeNull());
  socket.serverOpen();
  await vi.waitFor(() => expect(socket.lastOf('hello')).toBeDefined());
  socket.serverSend({ t: 'hello-ack', capabilities: [] });
};

const subscriber = () => {
  const events: string[] = [];
  let snapshot: Uint8Array | null = null;
  let snapshotSeq = '';
  const sub: ChannelSubscriber = {
    onSnapshot: (graph, meta) => {
      events.push('snapshot');
      snapshot = graph;
      snapshotSeq = meta.seq;
    },
    onChangesets: (event) => events.push(`changesets:${event.seq}`),
    onReload: () => events.push('reload'),
    onPermanentError: (code) => events.push(`error:${code}`),
  };
  return { sub, events, snapshot: () => snapshot, snapshotSeq: () => snapshotSeq };
};

const chunk = (documentId: string, rowId: string, seq: number, offset: number, bytes: Uint8Array) => ({
  t: 'snapshot-chunk',
  documentId,
  rowId,
  seq,
  offset,
  bytes,
});

describe('DocumentChannels', () => {
  test('스냅샷 축적: 청크 연접 후 snapshot-end에서 전달', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());

    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1, 2)));
    sockets[0].serverSend(chunk('D1', 'B1', 1, 2, Uint8Array.of(3)));
    sockets[0].serverSend(chunk('D1', 'B2', 2, 0, Uint8Array.of(4)));
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '5-0', heads: Uint8Array.of(9), durableHeads: Uint8Array.of(8) });

    await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(new Uint8Array(s.snapshot()!)).toEqual(Uint8Array.of(1, 2, 3, 4));
    expect(s.snapshotSeq()).toBe('5-0');
  });

  test('같은 행 offset 불연속은 축적 폐기 + fresh 재attach를 유발한다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());

    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1, 2)));
    sockets[0].serverSend(chunk('D1', 'B1', 1, 5, Uint8Array.of(9)));
    await vi.waitFor(() => expect(sockets[0].lastOf('detach')).toBeDefined());
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(2));

    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B9', 9, 0, Uint8Array.of(7, 7)));
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '9-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });

    await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(new Uint8Array(s.snapshot()!)).toEqual(Uint8Array.of(7, 7));
  });

  test('재개 수락: 첫 청크가 probe 좌표와 일치하면 이어붙인다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1, 2)));

    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 5000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].lastOf('attach')?.snapshotCursor).toEqual({ rowId: 'B1', seq: 1, offset: 2 }));

    sockets[1].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[1].serverSend(chunk('D1', 'B1', 1, 2, Uint8Array.of(3)));
    sockets[1].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '5-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });

    await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(new Uint8Array(s.snapshot()!)).toEqual(Uint8Array.of(1, 2, 3));
  });

  test('재개 거부: 첫 청크가 probe와 다르면 축적을 폐기하고 새 스트림으로 받는다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1, 2)));

    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 5000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].lastOf('attach')?.snapshotCursor).toEqual({ rowId: 'B1', seq: 1, offset: 2 }));

    sockets[1].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[1].serverSend(chunk('D1', 'B9', 9, 0, Uint8Array.of(7, 7)));
    sockets[1].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '9-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });

    await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(new Uint8Array(s.snapshot()!)).toEqual(Uint8Array.of(7, 7));
  });

  test('document transient 오류는 백오프 후 재attach한다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());

    sockets[0].serverSend({ t: 'error', scope: 'document', documentId: 'D1', code: 'internal', permanent: false });
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(2));
    expect(s.events).toEqual([]);
  });

  test('live changesets는 구독자에 전달되고 sinceSeq가 전진한다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    sockets[0].serverSend({
      t: 'changesets',
      documentId: 'D1',
      seq: '6-0',
      bundles: [Uint8Array.of(6)],
      heads: new Uint8Array(),
      durableHeads: new Uint8Array(),
    });
    await vi.waitFor(() => expect(s.events).toEqual(['snapshot', 'changesets:6-0']));

    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 3000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].lastOf('attach')).toBeDefined());
    expect(sockets[1].lastOf('attach')?.sinceSeq).toBe('6-0');
  });

  test('발신자용 빈 changesets 알림(bundles: [])도 sinceSeq를 전진시킨다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    sockets[0].serverSend({
      t: 'changesets',
      documentId: 'D1',
      seq: '6-0',
      bundles: [],
      heads: Uint8Array.of(9),
      durableHeads: Uint8Array.of(8),
    });
    await vi.waitFor(() => expect(s.events).toEqual(['snapshot', 'changesets:6-0']));

    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 3000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].lastOf('attach')).toBeDefined());
    expect(sockets[1].lastOf('attach')?.sinceSeq).toBe('6-0');
  });

  test('늦은 구독자: fresh attach의 과거 tail은 기존 구독자에게 전달되지 않는다 (구독자별 커서)', async () => {
    const { channels, sockets } = setup();
    const first = subscriber();
    channels.subscribe('D1', first.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '6-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    sockets[0].serverSend({
      t: 'changesets',
      documentId: 'D1',
      seq: '7-0',
      bundles: [Uint8Array.of(7)],
      heads: new Uint8Array(),
      durableHeads: new Uint8Array(),
    });
    await vi.waitFor(() => expect(first.events).toEqual(['snapshot', 'changesets:7-0']));

    const second = subscriber();
    channels.subscribe('D1', second.sub);
    await vi.waitFor(() => expect(sockets[0].lastOf('detach')).toBeDefined());
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(2));

    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1)));
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '3-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    sockets[0].serverSend({
      t: 'changesets',
      documentId: 'D1',
      seq: '5-0',
      bundles: [Uint8Array.of(5)],
      heads: new Uint8Array(),
      durableHeads: new Uint8Array(),
    });
    sockets[0].serverSend({
      t: 'changesets',
      documentId: 'D1',
      seq: '8-0',
      bundles: [Uint8Array.of(8)],
      heads: new Uint8Array(),
      durableHeads: new Uint8Array(),
    });

    await vi.waitFor(() => expect(second.events).toEqual(['snapshot', 'changesets:5-0', 'changesets:8-0']));
    expect(first.events).toEqual(['snapshot', 'changesets:7-0', 'changesets:8-0']);
  });

  test('reload·permanent error 전달, permanent는 채널 정리, 마지막 해제 시 detach', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    const off = channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'reload', documentId: 'D1' });
    await vi.waitFor(() => expect(s.events).toEqual(['reload']));

    sockets[0].serverSend({ t: 'error', scope: 'document', documentId: 'D1', code: 'forbidden', permanent: true });
    await vi.waitFor(() => expect(s.events).toEqual(['reload', 'error:forbidden']));
    sockets[0].serverSend({ t: 'reload', documentId: 'D1' });
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(s.events).toEqual(['reload', 'error:forbidden']);

    off();
    const s2 = subscriber();
    channels.subscribe('D1', s2.sub);
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBeGreaterThanOrEqual(2));
  });

  test('loadDocumentSnapshot: snapshot-end에서 resolve하고 구독을 해제한다 (one-shot)', async () => {
    const { channels, sockets } = setup();
    const promise = loadDocumentSnapshot(channels, 'D1');
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1, 2)));
    sockets[0].serverSend(chunk('D1', 'B1', 1, 2, Uint8Array.of(3)));
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '2-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    await expect(promise).resolves.toEqual(Uint8Array.of(1, 2, 3));
    await vi.waitFor(() => expect(sockets[0].lastOf('detach')).toBeDefined());
  });

  test('재시작 fence: 재attach 후 attach-ack 전에 도착한 구 세대 프레임은 폐기된다', async () => {
    const { channels, sockets } = setup();
    const first = subscriber();
    channels.subscribe('D1', first.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1)));

    const second = subscriber();
    channels.subscribe('D1', second.sub);
    await vi.waitFor(() => expect(sockets[0].lastOf('detach')).toBeDefined());

    sockets[0].serverSend(chunk('D1', 'B2', 2, 0, Uint8Array.of(9)));
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '2-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    sockets[0].serverSend({
      t: 'changesets',
      documentId: 'D1',
      seq: '9-9',
      bundles: [Uint8Array.of(2)],
      heads: new Uint8Array(),
      durableHeads: new Uint8Array(),
    });
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend(chunk('D1', 'B3', 3, 0, Uint8Array.of(5)));
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '3-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    await vi.waitFor(() => expect(second.events).toEqual(['snapshot']));
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(new Uint8Array(second.snapshot()!)).toEqual(Uint8Array.of(5));
    expect(first.events).toEqual(['snapshot']);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(new Uint8Array(first.snapshot()!)).toEqual(Uint8Array.of(5));

    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 5000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].lastOf('attach')?.sinceSeq).toBe('3-0'));
  });

  test('loadDocumentSnapshot: permanent error는 reject하고 구독을 해제한다', async () => {
    const { channels, sockets } = setup();
    const promise = loadDocumentSnapshot(channels, 'D1');
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'error', scope: 'document', documentId: 'D1', code: 'forbidden', permanent: true });
    await expect(promise).rejects.toThrow('forbidden');
  });

  test('permanent error는 대기 중인 transient 재시도 타이머를 정리해 재구독 후 좀비 attach를 막는다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());

    sockets[0].serverSend({ t: 'error', scope: 'document', documentId: 'D1', code: 'internal', permanent: false });
    sockets[0].serverSend({ t: 'error', scope: 'document', documentId: 'D1', code: 'forbidden', permanent: true });
    await vi.waitFor(() => expect(s.events).toEqual(['error:forbidden']));

    const s2 = subscriber();
    channels.subscribe('D1', s2.sub);
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(2));
    const attachCountAfterResubscribe = sockets[0].sent.filter((m) => m.t === 'attach').length;

    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(attachCountAfterResubscribe);
  });

  test('attach-ack 워치독: 응답 없는 attach는 probe 후 detach+재attach한다', async () => {
    vi.useFakeTimers();
    try {
      const { channels, sockets } = setup();
      const s = subscriber();
      channels.subscribe('D1', s.sub);
      await vi.waitFor(() => expect(sockets.length).toBe(1));
      sockets[0].serverOpen();
      await vi.waitFor(() => expect(sockets[0].lastOf('hello')).toBeDefined());
      sockets[0].serverSend({ t: 'hello-ack', capabilities: [] });
      await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(1));

      await vi.advanceTimersByTimeAsync(10_000);
      expect(sockets[0].lastOf('ping')).toBeDefined();
      sockets[0].serverSend({ t: 'pong' });
      await vi.advanceTimersByTimeAsync(0);

      expect(sockets[0].sent.filter((m) => m.t === 'detach').length).toBe(1);
      expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(2);

      sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
      sockets[0].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1)));
      sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '1-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
      await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));

      await vi.advanceTimersByTimeAsync(15_000);
      expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(2);
    } finally {
      vi.useRealTimers();
    }
  });

  test('attach-ack 워치독: 죽은 연결이면 소켓을 버리고 재연결에서 재attach한다', async () => {
    vi.useFakeTimers();
    try {
      const { channels, sockets } = setup();
      const s = subscriber();
      channels.subscribe('D1', s.sub);
      await vi.waitFor(() => expect(sockets.length).toBe(1));
      sockets[0].serverOpen();
      await vi.waitFor(() => expect(sockets[0].lastOf('hello')).toBeDefined());
      sockets[0].serverSend({ t: 'hello-ack', capabilities: [] });
      await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(1));
      sockets[0].closeCompletes = false;

      await vi.advanceTimersByTimeAsync(10_000);
      expect(sockets[0].lastOf('ping')).toBeDefined();
      await vi.advanceTimersByTimeAsync(5000);
      expect(sockets[0].closed).not.toBeNull();

      await vi.advanceTimersByTimeAsync(1000);
      expect(sockets.length).toBe(2);
      sockets[1].serverOpen();
      await vi.waitFor(() => expect(sockets[1].lastOf('hello')).toBeDefined());
      sockets[1].serverSend({ t: 'hello-ack', capabilities: [] });
      await vi.waitFor(() => expect(sockets[1].sent.filter((m) => m.t === 'attach').length).toBe(1));

      sockets[1].serverSend({ t: 'attach-ack', documentId: 'D1' });
      sockets[1].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1)));
      sockets[1].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '1-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
      await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));
    } finally {
      vi.useRealTimers();
    }
  });

  test('연결이 protocol error로 닫히면 구독자에 permanent 에러가 전파되고 재구독으로 회복한다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].sent.filter((m) => m.t === 'attach').length).toBe(1));

    sockets[0].serverClose(4003);
    await vi.waitFor(() => expect(s.events).toEqual(['error:connection_permanent_protocol_error']));

    const s2 = subscriber();
    channels.subscribe('D1', s2.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 3000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].sent.filter((m) => m.t === 'attach').length).toBe(1));
    sockets[1].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[1].serverSend(chunk('D1', 'B1', 1, 0, Uint8Array.of(1)));
    sockets[1].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '1-0', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    await vi.waitFor(() => expect(s2.events).toEqual(['snapshot']));
  });

  test('빈 문서(seq="") snapshot-end로 live가 된 채널은 재연결 시 sinceSeq: ""로 재attach한다', async () => {
    const { channels, sockets } = setup();
    const s = subscriber();
    channels.subscribe('D1', s.sub);
    await vi.waitFor(() => expect(sockets.length).toBe(1));
    await handshake(sockets[0]);
    await vi.waitFor(() => expect(sockets[0].lastOf('attach')).toBeDefined());
    sockets[0].serverSend({ t: 'attach-ack', documentId: 'D1' });
    sockets[0].serverSend({ t: 'snapshot-end', documentId: 'D1', seq: '', heads: new Uint8Array(), durableHeads: new Uint8Array() });
    await vi.waitFor(() => expect(s.events).toEqual(['snapshot']));

    sockets[0].serverClose(1006);
    await vi.waitFor(() => expect(sockets.length).toBe(2), { timeout: 3000 });
    await handshake(sockets[1]);
    await vi.waitFor(() => expect(sockets[1].lastOf('attach')).toBeDefined());
    expect(sockets[1].lastOf('attach')?.sinceSeq).toBe('');
  });
});

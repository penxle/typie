import assert from 'node:assert/strict';
import test from 'node:test';
import { createSseParser, parseStreamFrame, pumpSse, readUntilSync } from './prism-stream.ts';

const enc = (chunks: string[], hang = false) =>
  new ReadableStream<Uint8Array>({
    start(c) {
      for (const chunk of chunks) c.enqueue(new TextEncoder().encode(chunk));
      if (!hang) c.close();
    },
  });

const ev = (seq: number, kind: string, data: object, context: object | null = {}) =>
  `id: ${seq}\nevent: ${kind}\ndata: ${JSON.stringify({ seq, kind, occurredAt: 1000 + seq, context, data })}\n\n`;

test('SSE 파서는 분절 청크를 이어 붙이고, event 없는 프레임(주석)은 버린다', () => {
  const parser = createSseParser();
  assert.deepEqual(parser.feed('event: sync\ndata: {"se'), []);
  assert.deepEqual(parser.feed('q":3}\n\n: hb\n\n'), [{ event: 'sync', data: '{"seq":3}' }]);
});

test('parseStreamFrame은 프레임 4종을 판별하고, 깨진 프레임은 던진다', () => {
  assert.deepEqual(parseStreamFrame({ event: 'heartbeat', data: '{}' }), { type: 'heartbeat' });
  assert.deepEqual(parseStreamFrame({ event: 'sync', data: '{"seq":9}' }), { type: 'sync', seq: 9 });
  const delta = parseStreamFrame({
    event: 'turn.delta',
    data: JSON.stringify({
      context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 },
      channel: 'text',
      offset: 0,
      data: '안',
    }),
  });
  assert.equal(delta.type, 'delta');
  const frame = parseStreamFrame({
    event: 'run.started',
    data: JSON.stringify({
      seq: 2,
      kind: 'run.started',
      occurredAt: 5,
      context: { agent: { id: 'a', name: 'x' }, run: 1 },
      data: { message: '안녕' },
    }),
  });
  assert.equal(frame.type, 'event');
  assert.throws(() => parseStreamFrame({ event: 'run.started', data: '{broken' }));
  assert.throws(() => parseStreamFrame({ event: 'run.started', data: '{"kind":"run.started"}' }));
});

test('pumpSse: 프레임 순서 전달·closed, stopAtSync면 sync 뒤 멈춤', async () => {
  const kinds: string[] = [];
  const outcome = await pumpSse({
    stream: enc([ev(1, 'run.started', { message: 'a' }, { agent: { id: 'a', name: 'x' }, run: 1 }), 'event: sync\ndata: {"seq":1}\n\n']),
    onFrame: (f) => {
      kinds.push(f.type);
    },
    idleMs: 1000,
    signal: new AbortController().signal,
  });
  assert.deepEqual(kinds, ['event', 'sync']);
  assert.equal(outcome, 'closed');

  const early: string[] = [];
  await pumpSse({
    stream: enc(['event: sync\ndata: {"seq":0}\n\n', 'event: heartbeat\ndata: {}\n\n'], true),
    onFrame: (f) => {
      early.push(f.type);
    },
    idleMs: 1000,
    signal: new AbortController().signal,
    stopAtSync: true,
  });
  assert.deepEqual(early, ['sync']);
});

test('pumpSse: idleMs 무수신이면 idle, abort면 aborted', async () => {
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- the outcome is under test here, not the frames
  assert.equal(await pumpSse({ stream: enc([], true), onFrame: () => {}, idleMs: 20, signal: new AbortController().signal }), 'idle');
  const controller = new AbortController();
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- the outcome is under test here, not the frames
  const pending = pumpSse({ stream: enc([], true), onFrame: () => {}, idleMs: 5000, signal: controller.signal });
  controller.abort();
  assert.equal(await pending, 'aborted');
});

test('readUntilSync: sync까지의 이벤트와 high-water, sync 없이 끝나면 던진다', async () => {
  const result = await readUntilSync(
    enc(
      [
        ev(1, 'run.started', { message: 'a' }, { agent: { id: 'a', name: 'x' }, run: 1 }),
        ev(2, 'run.completed', {}, { agent: { id: 'a', name: 'x' }, run: 1 }),
        'event: sync\ndata: {"seq":2}\n\n',
        'event: heartbeat\ndata: {}\n\n',
      ],
      true,
    ),
    new AbortController().signal,
  );
  assert.deepEqual(
    result.events.map((e) => e.seq),
    [1, 2],
  );
  assert.equal(result.sync, 2);

  await assert.rejects(readUntilSync(enc([]), new AbortController().signal), /ended without sync/);
});

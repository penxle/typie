import assert from 'node:assert/strict';
import test from 'node:test';
import { DocumentChannel, SNAPSHOT_CHUNK_BYTES } from './channel.ts';
import { collectSend, FakeSyncDeps } from './testing.ts';

const makeChannel = (deps: FakeSyncDeps, documentId = 'D1', clientId = 'c1') => {
  const { sent, send } = collectSend();
  return { sent, channel: new DocumentChannel({ deps, send, documentId, clientId }) };
};

test('신규 attach: attach-ack → 청크 → snapshot-end(collectedSeq) → tail 순서', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [
    { id: 'B1', seq: 1, payload: Uint8Array.of(1, 1) },
    { id: 'B2', seq: 2, payload: Uint8Array.of(2, 2, 2) },
  ]);
  deps.collectedSeq.set('D1', '5-0');
  deps.seedStream('D1', [{ seq: '6-0', changeset: Uint8Array.of(6) }]);
  deps.liveHeads.set('D1', Uint8Array.of(0xaa));
  deps.durableHeadsMap.set('D1', Uint8Array.of(0xdd));

  const { sent, channel } = makeChannel(deps);
  await channel.start();

  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'snapshot-chunk', 'snapshot-chunk', 'snapshot-end', 'changesets'],
  );
  const end = sent[3];
  if (end.t !== 'snapshot-end') return assert.fail();
  assert.equal(end.seq, '5-0');
  assert.deepEqual(end.heads, Uint8Array.of(0xaa));
  assert.deepEqual(end.durableHeads, Uint8Array.of(0xdd));
  const tail = sent[4];
  if (tail.t !== 'changesets') return assert.fail();
  assert.equal(tail.seq, '6-0');
  assert.deepEqual(tail.bundles, [Uint8Array.of(6)]);
  channel.stop();
});

test('collectedSeq는 bundle 행 읽기 전에 확정된다', async () => {
  const deps = new FakeSyncDeps();
  deps.collectedSeq.set('D1', '5-0');
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1) }]);
  const originalRead = deps.readBundlesAfter;
  deps.readBundlesAfter = async (documentId, afterSeq, limit) => {
    deps.collectedSeq.set('D1', '9-0');
    return originalRead(documentId, afterSeq, limit);
  };
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  const end = sent.find((m) => m.t === 'snapshot-end');
  if (end?.t !== 'snapshot-end') return assert.fail();
  assert.equal(end.seq, '5-0');
  channel.stop();
});

test('대형 행은 SNAPSHOT_CHUNK_BYTES 단위로 분할된다', async () => {
  const deps = new FakeSyncDeps();
  const big = new Uint8Array(SNAPSHOT_CHUNK_BYTES * 2 + 100).fill(7);
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: big }]);
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  const chunks = sent.filter((m) => m.t === 'snapshot-chunk');
  assert.equal(chunks.length, 3);
  if (chunks[2].t !== 'snapshot-chunk') return assert.fail();
  assert.equal(chunks[2].offset, SNAPSHOT_CHUNK_BYTES * 2);
  assert.equal(chunks[2].bytes.length, 100);
  channel.stop();
});

test('빈 문서: 청크 없이 snapshot-end만, tail 없음', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'snapshot-end'],
  );
  const end = sent[1];
  if (end.t !== 'snapshot-end') return assert.fail();
  assert.equal(end.seq, '');
  assert.equal(end.heads.length, 0);
  channel.stop();
});

test('BUNDLE_READ_LIMIT 초과 행도 전부 스트림된다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles(
    'D1',
    Array.from({ length: 20 }, (_, i) => ({ id: `B${i + 1}`, seq: i + 1, payload: Uint8Array.of(i) })),
  );
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  assert.equal(sent.filter((m) => m.t === 'snapshot-chunk').length, 20);
  channel.stop();
});

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

const b64 = (bytes: Uint8Array) => bytes.toBase64();

const liveEvent = (payload: Uint8Array, target = '*') => ({
  target,
  seq: '7-0',
  changesets: [b64(payload)],
  heads: b64(Uint8Array.of(0xaa)),
  durableHeads: b64(Uint8Array.of(0xdd)),
});

test('snapshot 중 도착한 live 이벤트는 tail 뒤에 flush된다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1) }]);
  deps.seedStream('D1', [{ seq: '6-0', changeset: Uint8Array.of(6) }]);
  const originalRead = deps.readBundlesAfter;
  deps.readBundlesAfter = async (documentId, afterSeq, limit) => {
    deps.emit('D1', liveEvent(Uint8Array.of(7)));
    return originalRead(documentId, afterSeq, limit);
  };
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  await tick();
  const kinds = sent.map((m) => m.t);
  assert.deepEqual(kinds, ['attach-ack', 'snapshot-chunk', 'snapshot-end', 'changesets', 'changesets']);
  const flushed = sent[4];
  if (flushed.t !== 'changesets') return assert.fail();
  assert.deepEqual(
    flushed.bundles.map((b) => new Uint8Array(b)),
    [Uint8Array.of(7)],
  );
  channel.stop();
});

test('live 전환 후 이벤트는 즉시 전달된다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  deps.emit('D1', liveEvent(Uint8Array.of(9)));
  await tick();
  const last = sent.at(-1);
  if (last?.t !== 'changesets') return assert.fail();
  assert.equal(last.seq, '7-0');
  channel.stop();
});

test('echo 필터: !clientId는 자기 것만 걸러진다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps, 'D1', 'me');
  await channel.start();
  deps.emit('D1', liveEvent(Uint8Array.of(1), '!me'));
  deps.emit('D1', liveEvent(Uint8Array.of(2), '!other'));
  deps.emit('D1', liveEvent(Uint8Array.of(3), 'me'));
  deps.emit('D1', liveEvent(Uint8Array.of(4), 'other'));
  await tick();
  const delivered = sent.filter((m) => m.t === 'changesets');
  assert.equal(delivered.length, 2);
  channel.stop();
});

test('stop 후에는 live 이벤트가 전달되지 않고 구독이 해제된다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  channel.stop();
  const before = sent.length;
  deps.emit('D1', liveEvent(Uint8Array.of(9)));
  await tick();
  assert.equal(sent.length, before);
  assert.equal(channel.stopped, true);
});

test('collect의 빈 changesets 알림(seq 없음)도 heads 갱신용으로 전달된다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  deps.emit('D1', { target: '*', seq: '', changesets: [], heads: b64(Uint8Array.of(1)), durableHeads: b64(Uint8Array.of(1)) });
  await tick();
  const last = sent.at(-1);
  if (last?.t !== 'changesets') return assert.fail();
  assert.equal(last.seq, '');
  assert.deepEqual(last.bundles, []);
  channel.stop();
});

test('mid-row 재개: 정확히 (rowId, offset)부터 시작하고 이후 행 계속', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [
    { id: 'B1', seq: 1, payload: Uint8Array.of(1, 2, 3, 4) },
    { id: 'B2', seq: 2, payload: Uint8Array.of(5) },
  ]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ snapshotCursor: { rowId: 'B1', seq: 1, offset: 2 } });
  const chunks = sent.filter((m) => m.t === 'snapshot-chunk');
  assert.equal(chunks.length, 2);
  if (chunks[0].t !== 'snapshot-chunk' || chunks[1].t !== 'snapshot-chunk') return assert.fail();
  assert.equal(chunks[0].rowId, 'B1');
  assert.equal(chunks[0].offset, 2);
  assert.deepEqual(new Uint8Array(chunks[0].bytes), Uint8Array.of(3, 4));
  assert.equal(chunks[1].rowId, 'B2');
  channel.stop();
});

test('행 경계 재개: 빈 bytes 확인 청크 후 다음 행', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [
    { id: 'B1', seq: 1, payload: Uint8Array.of(1, 2) },
    { id: 'B2', seq: 2, payload: Uint8Array.of(3) },
  ]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ snapshotCursor: { rowId: 'B1', seq: 1, offset: 2 } });
  const chunks = sent.filter((m) => m.t === 'snapshot-chunk');
  if (chunks[0].t !== 'snapshot-chunk') return assert.fail();
  assert.equal(chunks[0].rowId, 'B1');
  assert.equal(chunks[0].offset, 2);
  assert.equal(chunks[0].bytes.length, 0);
  if (chunks[1].t !== 'snapshot-chunk') return assert.fail();
  assert.equal(chunks[1].rowId, 'B2');
  channel.stop();
});

test('rowId 부재(consolidation): 처음부터 전체 재스트림', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [{ id: 'B9', seq: 9, payload: Uint8Array.of(9, 9) }]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ snapshotCursor: { rowId: 'GONE', seq: 3, offset: 100 } });
  const chunks = sent.filter((m) => m.t === 'snapshot-chunk');
  assert.equal(chunks.length, 1);
  if (chunks[0].t !== 'snapshot-chunk') return assert.fail();
  assert.equal(chunks[0].rowId, 'B9');
  assert.equal(chunks[0].offset, 0);
  channel.stop();
});

test('sinceSeq 모드: snapshot 생략, tail만', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1) }]);
  deps.seedStream('D1', [
    { seq: '3-0', changeset: Uint8Array.of(3) },
    { seq: '4-0', changeset: Uint8Array.of(4) },
  ]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '3-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'changesets'],
  );
  const tail = sent[1];
  if (tail.t !== 'changesets') return assert.fail();
  assert.equal(tail.seq, '4-0');
  channel.stop();
});

test('sinceSeq 빈 문자열(빈 문서 기준) + 실제 trim 발생: reload 전송 후 채널 stop', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  deps.collectedSeq.set('D1', '9-0');
  deps.trimmed.add('D1');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('sinceSeq 빈 문자열(빈 문서 기준) + collect 미발생: 기존처럼 tail-resume(reload 없음)', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'changesets'],
  );
  channel.stop();
});

test('sinceSeq 빈 문자열 + collect 존재 + trim 없음(스트림 잔존)이면 reload 없이 tail 정상 전달', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  deps.collectedSeq.set('D1', '9-0');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'changesets'],
  );
  const tail = sent[1];
  if (tail.t !== 'changesets') return assert.fail();
  assert.equal(tail.seq, '9-0');
  assert.deepEqual(
    tail.bundles.map((b) => new Uint8Array(b)),
    [Uint8Array.of(9)],
  );
  channel.stop();
});

test('sinceSeq 빈 문자열 + collect 존재 + 스트림 빈 상태: reload 전송 후 채널 stop', async () => {
  const deps = new FakeSyncDeps();
  deps.collectedSeq.set('D1', '9-0');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('truncated: reload 전송 후 채널 stop', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  deps.oldestRetained.set('D1', '9-0');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '1-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('tail 배치: TAIL_BATCH_ENTRIES 단위로 분할', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream(
    'D1',
    Array.from({ length: 100 }, (_, i) => ({ seq: `${i + 1}-0`, changeset: Uint8Array.of(i % 256) })),
  );
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  const batches = sent.filter((m) => m.t === 'changesets');
  assert.equal(batches.length, 2);
  if (batches[0].t !== 'changesets' || batches[1].t !== 'changesets') return assert.fail();
  assert.equal(batches[0].bundles.length, 64);
  assert.equal(batches[0].seq, '64-0');
  assert.equal(batches[1].bundles.length, 36);
  channel.stop();
});

test('cursor의 rowId가 존재해도 seq가 불일치하면 전체 재스트림', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [{ id: 'B1', seq: 5, payload: Uint8Array.of(1, 2) }]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ snapshotCursor: { rowId: 'B1', seq: 3, offset: 1 } });
  const chunks = sent.filter((m) => m.t === 'snapshot-chunk');
  assert.equal(chunks.length, 1);
  if (chunks[0].t !== 'snapshot-chunk') return assert.fail();
  assert.equal(chunks[0].offset, 0);
  channel.stop();
});

test('tail 배치: 바이트 상한을 넘기 전에 flush된다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [
    { seq: '1-0', changeset: new Uint8Array(200 * 1024) },
    { seq: '2-0', changeset: new Uint8Array(100 * 1024) },
  ]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  const batches = sent.filter((m) => m.t === 'changesets');
  assert.equal(batches.length, 2);
  channel.stop();
});

test('tail의 heads는 snapshot-end 이후 재독된다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1) }]);
  deps.seedStream('D1', [{ seq: '2-0', changeset: Uint8Array.of(2) }]);
  let calls = 0;
  deps.getLiveHeads = async () => {
    calls += 1;
    return calls === 1 ? Uint8Array.of(0x01) : Uint8Array.of(0x02);
  };
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  const end = sent.find((m) => m.t === 'snapshot-end');
  const tail = sent.find((m) => m.t === 'changesets');
  if (end?.t !== 'snapshot-end' || tail?.t !== 'changesets') return assert.fail();
  assert.deepEqual(end.heads, Uint8Array.of(0x01));
  assert.deepEqual(tail.heads, Uint8Array.of(0x02));
  channel.stop();
});

test('미래 sinceSeq(tip 초과)는 reload', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '9999999999999-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('스트림이 완전히 비었는데 sinceSeq가 있으면 reload (연속성 검증 불가)', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '50-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('만료된 idle 스트림 + sinceSeq == collectedSeq: reload 없이 빈 tail로 live 전환', async () => {
  const deps = new FakeSyncDeps();
  deps.collectedSeq.set('D1', '9-0');
  deps.trimmed.add('D1');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '9-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack'],
  );
  assert.equal(channel.stopped, false);
  deps.emit('D1', liveEvent(Uint8Array.of(7)));
  await tick();
  assert.equal(sent.at(-1)?.t, 'changesets');
  channel.stop();
});

test('만료된 idle 스트림 + sinceSeq < collectedSeq: 만료로 소실된 구간이 있으므로 reload', async () => {
  const deps = new FakeSyncDeps();
  deps.collectedSeq.set('D1', '9-0');
  deps.trimmed.add('D1');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '5-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('만료된 idle 스트림 + sinceSeq > collectedSeq(비정상 커서): reload', async () => {
  const deps = new FakeSyncDeps();
  deps.collectedSeq.set('D1', '9-0');
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '12-0' });
  assert.deepEqual(
    sent.map((m) => m.t),
    ['attach-ack', 'reload'],
  );
  assert.equal(channel.stopped, true);
});

test('페이지 읽기와 검사 사이의 trim 경쟁은 reload로 귀결되고 갭은 전송되지 않는다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream(
    'D1',
    Array.from({ length: 130 }, (_, i) => ({ seq: `${i + 1}-0`, changeset: Uint8Array.of(1) })),
  );
  const original = deps.readStreamBatch;
  deps.readStreamBatch = async (documentId, sinceSeq, count) => {
    const page = await original(documentId, sinceSeq, count);
    deps.trimTo('D1', '80-0');
    return page;
  };
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '1-0' });
  assert.equal(sent.filter((m) => m.t === 'changesets').length, 0);
  assert.equal(sent.at(-1)?.t, 'reload');
  assert.equal(channel.stopped, true);
});

test('tail은 시작 시점 tip까지만 — 페이징 중 유입은 live 버퍼 몫', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream(
    'D1',
    Array.from({ length: 64 }, (_, i) => ({ seq: `${i + 1}-0`, changeset: Uint8Array.of(1) })),
  );
  const original = deps.readStreamBatch;
  deps.readStreamBatch = async (documentId, sinceSeq, count) => {
    const page = await original(documentId, sinceSeq, count);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- seeded above
    const entries = deps.stream.get('D1')!;
    for (let i = 0; i < 64; i += 1) entries.push({ seq: `${1000 + entries.length}-0`, changeset: Uint8Array.of(2) });
    return page;
  };
  const { sent, channel } = makeChannel(deps);
  await channel.start({ sinceSeq: '' });
  const delivered = sent.filter((m) => m.t === 'changesets').flatMap((m) => (m.t === 'changesets' ? m.bundles : []));
  assert.equal(delivered.length, 64);
  channel.stop();
});

test('live 버퍼 상한 초과 시 onOverload 호출 후 채널 stop', async () => {
  const deps = new FakeSyncDeps();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const original = deps.readBundlesAfter;
  deps.readBundlesAfter = async (documentId, afterSeq, limit) => {
    await gate;
    return original(documentId, afterSeq, limit);
  };
  const { send } = collectSend();
  let overloaded = false;
  const channel = new DocumentChannel({ deps, send, documentId: 'D1', clientId: 'c1', onOverload: () => (overloaded = true) });
  const started = channel.start();
  await tick();
  const big = 'a'.repeat(3 * 1024 * 1024);
  deps.emit('D1', { target: '*', seq: '1-0', changesets: [big], heads: '', durableHeads: '' });
  deps.emit('D1', { target: '*', seq: '2-0', changesets: [big], heads: '', durableHeads: '' });
  await tick();
  assert.equal(overloaded, true);
  assert.equal(channel.stopped, true);
  release();
  await started;
});

test('echo 이벤트는 live 버퍼에 계수되지 않는다', async () => {
  const deps = new FakeSyncDeps();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const original = deps.readBundlesAfter;
  deps.readBundlesAfter = async (documentId, afterSeq, limit) => {
    await gate;
    return original(documentId, afterSeq, limit);
  };
  const { send } = collectSend();
  let overloaded = false;
  const channel = new DocumentChannel({ deps, send, documentId: 'D1', clientId: 'me', onOverload: () => (overloaded = true) });
  const started = channel.start();
  await tick();
  const big = 'a'.repeat(3 * 1024 * 1024);
  deps.emit('D1', { target: '!me', seq: '1-0', changesets: [big], heads: '', durableHeads: '' });
  deps.emit('D1', { target: '!me', seq: '2-0', changesets: [big], heads: '', durableHeads: '' });
  await tick();
  assert.equal(overloaded, false);
  assert.equal(channel.stopped, false);
  release();
  await started;
  channel.stop();
});

test('attach-ack 전송 중 stop이 와도 구독이 누수되지 않는다', async () => {
  const deps = new FakeSyncDeps();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const send = async () => {
    await gate;
  };
  const channel = new DocumentChannel({ deps, send, documentId: 'D1', clientId: 'c1' });
  const started = channel.start();
  channel.stop();
  release();
  await started;
  assert.equal(deps.subscriberCount('D1'), 0);
});

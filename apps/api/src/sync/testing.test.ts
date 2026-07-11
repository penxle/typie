import assert from 'node:assert/strict';
import test from 'node:test';
import { FakeSyncDeps } from './testing.ts';

test('readStreamSince: sinceSeq 이후 entries와 truncated 판정', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [
    { seq: '3-0', changeset: Uint8Array.of(3) },
    { seq: '4-0', changeset: Uint8Array.of(4) },
  ]);
  const all = await deps.readStreamSince('D1', null);
  assert.equal(all.entries.length, 2);
  assert.equal(all.tip, '4-0');
  assert.equal(all.truncated, false);

  const partial = await deps.readStreamSince('D1', '3-0');
  assert.deepEqual(
    partial.entries.map((e) => e.seq),
    ['4-0'],
  );

  deps.oldestRetained.set('D1', '3-0');
  const stale = await deps.readStreamSince('D1', '1-0');
  assert.equal(stale.truncated, true);
});

test('subscribeChangesets: emit이 구독자에 전달되고 return()으로 종료', async () => {
  const deps = new FakeSyncDeps();
  const subscription = deps.subscribeChangesets('D1');
  const event = { target: '*', seq: '1-0', changesets: [], heads: '', durableHeads: '' };
  deps.emit('D1', event);
  const iterator = subscription[Symbol.asyncIterator]();
  const first = await iterator.next();
  assert.deepEqual(first.value, event);
  subscription.return();
  const second = await iterator.next();
  assert.equal(second.done, true);
});

test('readStreamBatch는 페이징, isStreamTruncated는 retained 경계 비교', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [
    { seq: '1-0', changeset: Uint8Array.of(1) },
    { seq: '2-0', changeset: Uint8Array.of(2) },
    { seq: '3-0', changeset: Uint8Array.of(3) },
  ]);
  const page = await deps.readStreamBatch('D1', '1-0', 2);
  assert.deepEqual(
    page.map((e) => e.seq),
    ['2-0', '3-0'],
  );
  assert.equal(await deps.streamTip('D1'), '3-0');
  deps.oldestRetained.set('D1', '3-0');
  assert.equal(await deps.isStreamTruncated('D1', '1-0'), true);
  assert.equal(await deps.isStreamTruncated('D1', '3-0'), false);
});

test('appendBundle은 단조 seq를 발급하고 stream에 쌓는다', async () => {
  const deps = new FakeSyncDeps();
  const s1 = await deps.appendBundle('D1', Uint8Array.of(1), 'U1', 'DEV1');
  const s2 = await deps.appendBundle('D1', Uint8Array.of(2), 'U1', 'DEV1');
  assert.ok(s1 < s2);
  const { entries } = await deps.readStreamSince('D1', null);
  assert.equal(entries.length, 2);
});

test('advanceLiveHeads는 cold cache에서 null, warm에서 갱신', async () => {
  const deps = new FakeSyncDeps();
  assert.equal(await deps.advanceLiveHeads('D1', Uint8Array.of(1)), null);
  deps.liveHeads.set('D1', Uint8Array.of(9));
  const next = await deps.advanceLiveHeads('D1', Uint8Array.of(1, 2));
  assert.ok(next);
  assert.deepEqual(deps.liveHeads.get('D1'), next);
});

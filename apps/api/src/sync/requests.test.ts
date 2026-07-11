import assert from 'node:assert/strict';
import test from 'node:test';
import { handlePull, handlePush } from './requests.ts';
import { collectSend, FakeSyncDeps } from './testing.ts';
import type { RequestContext } from './requests.ts';

const ctx = (deps: FakeSyncDeps): RequestContext => ({
  deps,
  session: { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' },
  clientId: 'me',
});

test('push: append → advance → publish(!clientId) → collect enqueue → ack', async () => {
  const deps = new FakeSyncDeps();
  deps.liveHeads.set('D1', Uint8Array.of(0x01));
  deps.durableHeadsMap.set('D1', Uint8Array.of(0x02));
  const { sent, send } = collectSend();
  await handlePush(ctx(deps), { id: 'r1', documentId: 'D1', changesets: Uint8Array.of(9, 9) }, send);

  assert.equal(deps.published.length, 1);
  assert.equal(deps.published[0].event.target, '!me');
  assert.deepEqual(deps.collectJobs, ['D1']);
  const ack = sent[0];
  if (ack.t !== 'push-ack') return assert.fail();
  assert.equal(ack.id, 'r1');
  assert.deepEqual(ack.heads, deps.liveHeads.get('D1'));
  assert.deepEqual(ack.durableHeads, Uint8Array.of(0x02));
  const { entries } = await deps.readStreamSince('D1', null);
  assert.equal(entries.length, 1);
});

test('push: 잘못된 페이로드는 permanent 오류, append 없음', async () => {
  const deps = new FakeSyncDeps();
  const bad = Uint8Array.of(0xde, 0xad);
  deps.invalidPayloads.push(bad);
  const { sent, send } = collectSend();
  await handlePush(ctx(deps), { id: 'r1', documentId: 'D1', changesets: bad }, send);
  const error = sent[0];
  if (error.t !== 'error') return assert.fail();
  assert.equal(error.code, 'invalid_changeset_payload');
  assert.equal(error.permanent, true);
  assert.equal(error.id, 'r1');
  assert.equal(deps.published.length, 0);
  const { entries } = await deps.readStreamSince('D1', null);
  assert.equal(entries.length, 0);
});

test('push: opsCount 0(빈 번들)은 append/publish 없이 현재 heads로 ack', async () => {
  const deps = new FakeSyncDeps();
  deps.liveHeads.set('D1', Uint8Array.of(0x01));
  const { sent, send } = collectSend();
  await handlePush(ctx(deps), { id: 'r1', documentId: 'D1', changesets: new Uint8Array() }, send);
  assert.equal(deps.published.length, 0);
  assert.equal(deps.collectJobs.length, 0);
  const ack = sent[0];
  if (ack.t !== 'push-ack') return assert.fail();
  assert.deepEqual(ack.heads, Uint8Array.of(0x01));
});

test('push: cold cache면 bootstrapLiveHeads로 부트스트랩', async () => {
  const deps = new FakeSyncDeps();
  const { sent, send } = collectSend();
  await handlePush(ctx(deps), { id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1) }, send);
  const ack = sent[0];
  if (ack.t !== 'push-ack') return assert.fail();
  assert.deepEqual(ack.heads, Uint8Array.of(0xb0));
  assert.equal(deps.published.length, 1);
});

test('pull: sinceSeq 이후 tail과 커서 반환', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [
    { seq: '3-0', changeset: Uint8Array.of(3) },
    { seq: '4-0', changeset: Uint8Array.of(4) },
  ]);
  deps.liveHeads.set('D1', Uint8Array.of(0xaa));
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r2', documentId: 'D1', sinceSeq: '3-0' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, false);
  assert.equal(ack.seq, '4-0');
  assert.deepEqual(
    ack.changesets.map((c) => new Uint8Array(c)),
    [Uint8Array.of(4)],
  );
});

test('pull: truncated면 needsReload=true, changesets 비움', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  deps.oldestRetained.set('D1', '9-0');
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r2', documentId: 'D1', sinceSeq: '1-0' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, true);
  assert.deepEqual(ack.changesets, []);
  assert.equal(ack.seq, '1-0');
});

test('pull: tail이 상한을 넘으면 needsReload', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream(
    'D1',
    Array.from({ length: 65 }, (_, i) => ({ seq: `${i + 1}-0`, changeset: Uint8Array.of(1) })),
  );
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r3', documentId: 'D1' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, true);
  assert.deepEqual(ack.changesets, []);
});

test('pull: 미래 sinceSeq는 needsReload', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r4', documentId: 'D1', sinceSeq: '9999999999999-0' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, true);
});

test('pull: 빈 스트림에 sinceSeq가 있으면 needsReload', async () => {
  const deps = new FakeSyncDeps();
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r6', documentId: 'D1', sinceSeq: '50-0' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, true);
});

test('pull: 빈 문자열 sinceSeq(빈 문서 기준) + collect 발생이면 needsReload', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  deps.collectedSeq.set('D1', '9-0');
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r7', documentId: 'D1', sinceSeq: '' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, true);
  assert.deepEqual(ack.changesets, []);
});

test('pull: 빈 문자열 sinceSeq(빈 문서 기준) + collect 미발생이면 기존처럼 tail 반환', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '9-0', changeset: Uint8Array.of(9) }]);
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r8', documentId: 'D1', sinceSeq: '' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, false);
  assert.equal(ack.seq, '9-0');
  assert.deepEqual(
    ack.changesets.map((c) => new Uint8Array(c)),
    [Uint8Array.of(9)],
  );
});

test('pull: 빈 문자열 sinceSeq는 커서 없음으로 정규화된다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedStream('D1', [{ seq: '2-0', changeset: Uint8Array.of(2) }]);
  const { sent, send } = collectSend();
  await handlePull(ctx(deps), { id: 'r5', documentId: 'D1', sinceSeq: '' }, send);
  const ack = sent[0];
  if (ack.t !== 'pull-ack') return assert.fail();
  assert.equal(ack.needsReload, false);
  assert.equal(ack.seq, '2-0');
});

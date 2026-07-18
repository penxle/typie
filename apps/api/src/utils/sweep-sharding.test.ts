import assert from 'node:assert/strict';
import test from 'node:test';
import { mergeCommentHits, mergeReportEntries, shardOf } from './sweep-sharding.ts';
import type { SweepCommentHit, SweepReportEntry } from './sweep-sharding.ts';

const sampleIds = Array.from({ length: 20_000 }, (_, i) => `doc${(i * 2_654_435_761) % 1_000_000_007}x${i}`);

test('shardOf is deterministic across calls', () => {
  for (const id of sampleIds.slice(0, 200)) {
    assert.equal(shardOf(id, 16), shardOf(id, 16));
  }
});

test('shardOf stays within [0, workers)', () => {
  for (const workers of [1, 3, 8, 15, 64]) {
    for (const id of sampleIds.slice(0, 500)) {
      const shard = shardOf(id, workers);
      assert.ok(Number.isSafeInteger(shard) && shard >= 0 && shard < workers, `${id} @ ${workers} -> ${shard}`);
    }
  }
});

test('shardOf keeps every id on its shard when N is unchanged (resume stability)', () => {
  const workers = 8;
  const assignment = new Map(sampleIds.slice(0, 2000).map((id) => [id, shardOf(id, workers)]));
  // A resumed run re-derives the same assignment for the same N, so per-shard checkpoints stay valid.
  for (const [id, shard] of assignment) {
    assert.equal(shardOf(id, workers), shard);
  }
});

test('shardOf remaps a meaningful fraction of ids when N changes (justifies the N-mismatch guard)', () => {
  const remapped = sampleIds.slice(0, 2000).filter((id) => shardOf(id, 8) !== shardOf(id, 9)).length;
  // A different worker count genuinely moves documents between shards — resuming under the wrong N
  // would misassign them, which is why the orchestrator aborts on an N mismatch instead of resuming.
  assert.ok(remapped > 2000 * 0.5, `only ${remapped}/2000 ids remapped between N=8 and N=9`);
});

test('shardOf distributes roughly evenly over many ids', () => {
  const workers = 16;
  const counts = Array.from({ length: workers }, () => 0);
  for (const id of sampleIds) {
    counts[shardOf(id, workers)] += 1;
  }
  const expected = sampleIds.length / workers;
  for (const [shard, count] of counts.entries()) {
    assert.ok(count > 0, `shard ${shard} received no documents`);
    assert.ok(count > expected * 0.7 && count < expected * 1.3, `shard ${shard} skewed: ${count} vs ~${expected}`);
  }
});

test('mergeReportEntries dedupes by documentId, last wins', () => {
  const existing: SweepReportEntry[] = [
    { documentId: 'a', reason: 'deferred' },
    { documentId: 'b', reason: 'failed', message: 'old' },
  ];
  const incoming: SweepReportEntry[] = [
    { documentId: 'b', reason: 'failed', message: 'new' },
    { documentId: 'c', reason: 'deferred' },
  ];
  const merged = mergeReportEntries(existing, incoming);
  assert.equal(merged.length, 3);
  assert.deepEqual(
    merged.find((e) => e.documentId === 'b'),
    { documentId: 'b', reason: 'failed', message: 'new' },
  );
  assert.ok(merged.some((e) => e.documentId === 'a'));
  assert.ok(merged.some((e) => e.documentId === 'c'));
});

test('mergeCommentHits dedupes by (documentId, threadId)', () => {
  const existing: SweepCommentHit[] = [
    { documentId: 'a', threadId: 't1', hitDots: ['x'] },
    { documentId: 'a', threadId: 't2', hitDots: null },
  ];
  const incoming: SweepCommentHit[] = [
    { documentId: 'a', threadId: 't1', hitDots: ['x', 'y'] },
    { documentId: 'b', threadId: 't1', hitDots: ['z'] },
  ];
  const merged = mergeCommentHits(existing, incoming);
  assert.equal(merged.length, 3);
  assert.deepEqual(merged.find((h) => h.documentId === 'a' && h.threadId === 't1')?.hitDots, ['x', 'y']);
});

test('mergeCommentHits does not collide (documentId, threadId) pairs that share a concatenated prefix', () => {
  const hits: SweepCommentHit[] = [
    { documentId: 'ab', threadId: 'c', hitDots: ['1'] },
    { documentId: 'a', threadId: 'bc', hitDots: ['2'] },
  ];
  const merged = mergeCommentHits([], hits);
  assert.equal(merged.length, 2, 'the separator must keep "ab|c" and "a|bc" distinct');
});

test('the merge helpers are identities on empty input', () => {
  assert.deepEqual(mergeReportEntries([], []), []);
  assert.deepEqual(mergeCommentHits([], []), []);
  const report: SweepReportEntry[] = [{ documentId: 'a', reason: 'failed' }];
  assert.deepEqual(mergeReportEntries(report, []), report);
  assert.deepEqual(mergeReportEntries([], report), report);
});

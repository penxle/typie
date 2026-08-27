import assert from 'node:assert/strict';
import test from 'node:test';
import { planHeadWrites } from './head-segmentation.ts';
import type { FoldedEntry, LatestHead } from './head-segmentation.ts';

const SYSTEM = 'system';
const PRISM = 'prism';
const heads = (n: number) => Uint8Array.of(n);
const entry = (over: Partial<FoldedEntry>): FoldedEntry => ({
  userId: 'u1',
  applied: true,
  charCount: 0,
  grossInsertions: 0,
  grossDeletions: 0,
  heads: heads(0),
  ...over,
});
const latest = (over: Partial<LatestHead> = {}): LatestHead => ({
  id: 'H0',
  kind: 'NORMAL',
  bucketMs: 1000,
  hasExcludedContributor: false,
  ...over,
});
const base = { baseCharCount: 100, bucketMs: 1000, systemUserId: SYSTEM, prismUserId: PRISM };

test('같은 bucket의 NORMAL 최신 행에는 접는다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [entry({ charCount: 120, grossInsertions: 20, heads: heads(1) })],
  });
  assert.deepEqual(writes, [
    {
      action: 'update',
      headId: 'H0',
      kind: 'NORMAL',
      heads: heads(1),
      characterCount: 120,
      isolatedAuthorId: null,
      contributions: [{ userId: 'u1', additions: 20, deletions: 0 }],
      contributorUserIds: ['u1'],
    },
  ]);
});

test('bucket이 다르면 새 NORMAL 행을 연다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest({ bucketMs: 400 }),
    entries: [entry({ charCount: 120, grossInsertions: 20 })],
  });
  assert.equal(writes[0].action, 'insert');
  assert.equal(writes[0].kind, 'NORMAL');
});

test('gross 임계 초과 엔트리는 ISOLATED 행으로 분리하고 이후는 새 행을 연다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [
      entry({ charCount: 110, grossInsertions: 10, heads: heads(1) }),
      entry({ charCount: 610, grossInsertions: 500, heads: heads(2) }),
      entry({ charCount: 615, grossInsertions: 5, heads: heads(3) }),
    ],
  });
  assert.deepEqual(
    writes.map((w) => [w.action, w.kind]),
    [
      ['update', 'NORMAL'],
      ['insert', 'ISOLATED'],
      ['insert', 'NORMAL'],
    ],
  );
  assert.equal(writes[1].isolatedAuthorId, 'u1');
  assert.equal(writes[1].characterCount, 610);
  assert.deepEqual(writes[1].heads, heads(2));
  assert.deepEqual(writes[1].contributions, [{ userId: 'u1', additions: 500, deletions: 0 }]);
  assert.deepEqual(writes[2].contributions, [{ userId: 'u1', additions: 5, deletions: 0 }]);
});

test('교체-붙여넣기(net 소폭 음수)도 gross 삭제로 격리한다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: null,
    entries: [entry({ charCount: 80, grossInsertions: 480, grossDeletions: 500 })],
  });
  assert.equal(writes[0].kind, 'ISOLATED');
  assert.deepEqual(writes[0].contributions, [{ userId: 'u1', additions: 0, deletions: 20 }]);
});

test('첫 엔트리부터 격리면 update 없이 ISOLATED 단독, 후속은 새 NORMAL', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [entry({ charCount: 600, grossInsertions: 500 }), entry({ charCount: 605, grossInsertions: 5 })],
  });
  assert.deepEqual(
    writes.map((w) => [w.action, w.kind]),
    [
      ['insert', 'ISOLATED'],
      ['insert', 'NORMAL'],
    ],
  );
});

test('excluded contributor가 있는 최신 행은 봉인 — 접지 않고 새 행', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest({ hasExcludedContributor: true }),
    entries: [entry({ charCount: 120, grossInsertions: 20 })],
  });
  assert.equal(writes[0].action, 'insert');
});

test('SYSTEM 엔트리는 격리하지 않고 contributor에서 제외하되 contributions에는 남는다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [entry({ userId: SYSTEM, charCount: 90, grossInsertions: 999, grossDeletions: 999 })],
  });
  assert.equal(writes[0].kind, 'NORMAL');
  assert.deepEqual(writes[0].contributorUserIds, []);
  assert.deepEqual(writes[0].contributions, [{ userId: SYSTEM, additions: 0, deletions: 10 }]);
});

test('세그먼트 내 같은 유저의 net은 상쇄 합산되고, 미적용 엔트리는 무시된다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [
      entry({ charCount: 200, grossInsertions: 100, heads: heads(1) }),
      entry({ applied: false, charCount: 200, heads: heads(1) }),
      entry({ charCount: 120, grossDeletions: 80, heads: heads(2) }),
    ],
  });
  assert.equal(writes.length, 1);
  assert.deepEqual(writes[0].contributions, [{ userId: 'u1', additions: 20, deletions: 0 }]);
  assert.deepEqual(writes[0].heads, heads(2));
});

test('net 0 유저는 contributions에서 빠지지만 contributor 행 대상에는 남는다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [entry({ charCount: 150, grossInsertions: 50 }), entry({ charCount: 100, grossDeletions: 50 })],
  });
  assert.deepEqual(writes[0].contributions, []);
  assert.deepEqual(writes[0].contributorUserIds, ['u1']);
});

test('미적용 엔트리는 gross가 커도 행을 만들지 않고, 다중 유저 net은 유저별로 분해된다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [
      entry({ userId: 'u1', charCount: 180, grossInsertions: 80, heads: heads(1) }),
      entry({ userId: 'u2', applied: false, charCount: 999, grossInsertions: 500, heads: heads(9) }),
      entry({ userId: 'u2', charCount: 150, grossDeletions: 30, heads: heads(2) }),
    ],
  });
  assert.equal(writes.length, 1);
  assert.equal(writes[0].action, 'update');
  assert.equal(writes[0].kind, 'NORMAL');
  assert.deepEqual(writes[0].heads, heads(2));
  assert.deepEqual(writes[0].contributions, [
    { userId: 'u1', additions: 80, deletions: 0 },
    { userId: 'u2', additions: 0, deletions: 30 },
  ]);
  assert.deepEqual(writes[0].contributorUserIds, ['u1', 'u2']);
});

test('PRISM 엔트리는 크기와 무관하게 앞 세그먼트를 접고 자기 ISOLATED 행을 연다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [
      entry({ charCount: 110, grossInsertions: 10, heads: heads(1) }),
      entry({ userId: PRISM, charCount: 112, grossInsertions: 2, heads: heads(2) }),
      entry({ charCount: 120, grossInsertions: 8, heads: heads(3) }),
    ],
  });
  assert.deepEqual(
    writes.map((w) => [w.action, w.kind]),
    [
      ['update', 'NORMAL'],
      ['insert', 'ISOLATED'],
      ['insert', 'NORMAL'],
    ],
  );
  assert.deepEqual(writes[0].heads, heads(1));
  assert.equal(writes[1].isolatedAuthorId, PRISM);
  assert.deepEqual(writes[1].contributorUserIds, [PRISM]);
  assert.deepEqual(writes[1].contributions, [{ userId: PRISM, additions: 2, deletions: 0 }]);
});

test('배치가 PRISM 엔트리로 시작하면 접을 세그먼트 없이 ISOLATED 행만 연다', () => {
  const writes = planHeadWrites({
    ...base,
    latestHead: latest(),
    entries: [entry({ userId: PRISM, charCount: 101, grossInsertions: 1, heads: heads(1) })],
  });
  assert.deepEqual(
    writes.map((w) => [w.action, w.kind]),
    [['insert', 'ISOLATED']],
  );
});

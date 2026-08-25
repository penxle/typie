import assert from 'node:assert/strict';
import test from 'node:test';
import dayjs from 'dayjs';
import {
  ConfirmInputSchema,
  confirmResult,
  detailOutcome,
  hasDetail,
  manuscriptPath,
  pickVersion,
  roundState,
  summarizeOutcome,
  threadsFromResult,
} from './prism-review-core.ts';
import type { ReviewOutcome } from '@typie/prism';

test('summarizeOutcome: feedback은 집계, issues는 건수만, rejected는 문면, null은 전부 비움', () => {
  const summarized = summarizeOutcome({
    version: 1,
    kind: 'feedback',
    issues: [{ trait: 't', pass: 'judgment', body: null, anchors: [] }],
    conclusion: {
      understanding: '리드',
      patterns: [{ theme: null, body: 'p', issues: [] }],
      priorities: [],
      strengths: [{ anchors: [{ start: 0, end: 1, head: 'h', tail: 't' }], body: null }],
    },
    verdicts: [{ trait: 't', point: 3, note: null }],
  });
  assert.deepEqual(summarized, {
    rejection: null,
    conclusion: {
      understanding: '리드',
      progress: null,
      strengthsCount: 1,
      verdictsCount: 1,
      elevationsCount: 0,
      patternsCount: 1,
      prioritiesCount: 0,
    },
    issueCount: 1,
  });
  assert.deepEqual(summarizeOutcome({ version: 1, kind: 'issues', issues: [] }), { rejection: null, conclusion: null, issueCount: 0 });
  assert.deepEqual(summarizeOutcome({ version: 1, kind: 'rejected', rejected: { category: 'diary', message: '안내', basis: null } }), {
    rejection: { message: '안내' },
    conclusion: null,
    issueCount: 0,
  });
  assert.deepEqual(summarizeOutcome(null), { rejection: null, conclusion: null, issueCount: 0 });
});

test('pickVersion: 최신과 content·title·subtitle 전부 같으면 재사용, 아니면 max+1', () => {
  const snap = { content: 'c', title: 't', subtitle: null };
  assert.deepEqual(pickVersion(null, snap), { reuse: false, version: 1 });
  assert.deepEqual(pickVersion({ version: 3, ...snap }, snap), { reuse: true, version: 3 });
  assert.deepEqual(pickVersion({ version: 3, content: 'c', title: 't2', subtitle: null }, snap), { reuse: false, version: 4 });
  assert.deepEqual(pickVersion({ version: 3, content: 'c2', title: 't', subtitle: null }, snap), { reuse: false, version: 4 });
});

test('roundState: 워크플로가 있으면 그 상태, 없으면 closedAt으로 CANCELED·PENDING', () => {
  assert.equal(roundState({ closedAt: null }, null), 'PENDING');
  assert.equal(roundState({ closedAt: dayjs() }, null), 'CANCELED');
  assert.equal(roundState({ closedAt: null }, { state: 'RUNNING' }), 'RUNNING');
  assert.equal(roundState({ closedAt: null }, { state: 'COMPLETED' }), 'COMPLETED');
  assert.equal(roundState({ closedAt: null }, { state: 'FAILED' }), 'FAILED');
  assert.equal(roundState({ closedAt: dayjs() }, { state: 'COMPLETED' }), 'COMPLETED');
});

test('confirmResult·manuscriptPath: prism 확인 결과 형태 그대로', () => {
  assert.equal(manuscriptPath('PRDV1'), 'manuscript/PRDV1.txt');
  assert.deepEqual(confirmResult('PRRR1', 'high', { id: 'D0DOC1', title: '제목', subtitle: null, path: 'manuscript/PRDV1.txt' }), {
    decision: 'confirmed',
    key: 'PRRR1',
    tier: 'high',
    document: { id: 'D0DOC1', title: '제목', subtitle: null, path: 'manuscript/PRDV1.txt' },
  });
});

test('ConfirmInputSchema: declined는 단독, confirmed는 documentId·대문자 tier 필수', () => {
  assert.deepEqual(ConfirmInputSchema.parse({ decision: 'declined' }), { decision: 'declined' });
  assert.deepEqual(ConfirmInputSchema.parse({ decision: 'confirmed', documentId: 'DOCU1', tier: 'HIGH' }), {
    decision: 'confirmed',
    documentId: 'DOCU1',
    tier: 'HIGH',
  });
  assert.equal(ConfirmInputSchema.safeParse({ decision: 'confirmed', documentId: 'DOCU1', tier: 'high' }).success, false);
  assert.equal(ConfirmInputSchema.safeParse({ decision: 'confirmed', documentId: 'DOCU1' }).success, false);
  assert.equal(ConfirmInputSchema.safeParse({ decision: 'confirmed', tier: 'LOW' }).success, false);
  assert.equal(ConfirmInputSchema.safeParse({ decision: 'maybe' }).success, false);
  assert.equal(ConfirmInputSchema.safeParse(null).success, false);
});

const feedback = {
  version: 1,
  kind: 'feedback',
  issues: [
    { id: 'judgment-1', trait: '인물', pass: 'judgment', body: '본문1', anchors: [] },
    { id: 'stylistic-1', trait: '문장', pass: 'stylistic', body: '본문2', anchors: [] },
  ],
  conclusion: {
    understanding: '리드',
    progress: '나아진 점',
    strengths: [{ anchors: [{ start: 0, end: 5, head: '가나다', tail: '다라마' }], body: '강점 설명' }],
    patterns: [{ theme: '주제', body: '습관', issues: ['stylistic-1', 'judgment-1'] }],
    priorities: [{ body: '순서', issues: ['judgment-1'] }],
  },
  verdicts: [
    { trait: '인물', point: 3, note: '메모' },
    { trait: '구성', point: 9, note: null },
  ],
  elevations: [{ trait: '문체', body: '격상 설명', anchors: [{ start: 0, end: 5, head: '', tail: '' }] }],
} satisfies ReviewOutcome;

test('detailOutcome: 총평 전문을 인용·해소된 참조로 사영한다', () => {
  const detail = detailOutcome(feedback, '가나다라마');
  assert.equal(detail?.understanding, '리드');
  assert.equal(detail?.progress, '나아진 점');
  assert.deepEqual(detail?.strengths, [
    { quote: '가나다라마', body: '강점 설명', anchors: [{ start: 0, end: 5, head: '가나다', tail: '다라마' }] },
  ]);
  assert.deepEqual(detail?.verdicts, [
    { trait: '인물', note: '메모' },
    { trait: '구성', note: null },
  ]);
  assert.deepEqual(detail?.elevations, [{ trait: '문체', quote: '가나다라마', body: '격상 설명' }]);
  assert.deepEqual(detail?.patterns, [
    {
      theme: '주제',
      body: '습관',
      issues: [
        { index: 1, trait: '문장' },
        { index: 0, trait: '인물' },
      ],
    },
  ]);
  assert.deepEqual(detail?.priorities, [{ body: '순서', issues: [{ index: 0, trait: '인물' }] }]);
});

test('detailOutcome: 강점 인용은 못 찾으면 머리·꼬리로 서고, 격상 인용은 없으면 빠진다', () => {
  const detail = detailOutcome({ ...feedback, elevations: [{ trait: '문체', body: '격상', anchors: [] }] }, '');
  assert.deepEqual(detail?.strengths, [
    { quote: '가나다 ⋯ 다라마', body: '강점 설명', anchors: [{ start: 0, end: 5, head: '가나다', tail: '다라마' }] },
  ]);
  assert.deepEqual(detail?.elevations, [{ trait: '문체', quote: null, body: '격상' }]);
});

test('detailOutcome: 참조는 번호로도 오고, 못 찾은 참조는 접히고, 같은 지적은 한 번만 선다', () => {
  // 구 결과의 번호 참조 — 타입은 문자열이지만 저장된 jsonb에는 숫자가 남아 있다.
  const legacy = {
    ...feedback,
    conclusion: {
      ...feedback.conclusion,
      patterns: [{ theme: null, body: '습관', issues: [1, 1, 9, 'judgment-1', 'judgment-9'] }],
      priorities: [],
    },
  } as unknown as ReviewOutcome;

  const detail = detailOutcome(legacy, '가나다라마');
  assert.deepEqual(detail?.patterns[0].issues, [
    { index: 1, trait: '문장' },
    { index: 0, trait: '인물' },
  ]);
});

test('detailOutcome: 총평이 없는 결과는 전부 null이다', () => {
  assert.equal(detailOutcome({ version: 1, kind: 'issues', issues: [] }, '본문'), null);
  assert.equal(
    detailOutcome({ version: 1, kind: 'rejected', rejected: { category: 'diary', message: '안내', basis: null } }, '본문'),
    null,
  );
  assert.equal(detailOutcome(null, '본문'), null);
});

test('hasDetail: 설 섹션이 하나라도 있어야 참이다', () => {
  const bare = {
    version: 1,
    kind: 'feedback',
    issues: [],
    conclusion: { understanding: null, patterns: [], priorities: [] },
  } satisfies ReviewOutcome;

  assert.equal(hasDetail(bare), false);
  assert.equal(hasDetail({ ...bare, conclusion: { ...bare.conclusion, understanding: ' '.repeat(3) } }), false);
  assert.equal(hasDetail({ ...bare, conclusion: { ...bare.conclusion, understanding: '리드' } }), true);
  assert.equal(hasDetail({ ...bare, verdicts: [{ trait: 't', point: 1, note: null }] }), true);
  assert.equal(hasDetail(feedback), true);
  assert.equal(hasDetail({ version: 1, kind: 'issues', issues: [] }), false);
  assert.equal(hasDetail(null), false);
});

test('threadsFromResult - 거부 회차와 빈 결과는 행이 없다', () => {
  assert.deepEqual(threadsFromResult(null), []);
  assert.deepEqual(threadsFromResult({ version: 1, kind: 'rejected', rejected: { category: 'diary', message: '거절', basis: null } }), []);
});

test('threadsFromResult - 이슈마다 번호와 계열을 붙인다', () => {
  const outcome = {
    version: 1,
    kind: 'issues',
    issues: [
      {
        id: 'i-1',
        trait: '동어 반복',
        pass: 'stylistic',
        body: '본문 설명',
        anchors: [{ start: 0, end: 3, head: '가나다', tail: '가나다' }],
      },
      { trait: '시점 흔들림', pass: 'judgment', body: null, anchors: [] },
    ],
  } satisfies ReviewOutcome;

  const threads = threadsFromResult(outcome);

  assert.equal(threads.length, 2);
  assert.equal(threads[0].issueIndex, 0);
  assert.equal(threads[0].issueId, 'i-1');
  assert.equal(threads[0].pass, 'STYLISTIC');
  assert.deepEqual(threads[0].anchors, [{ start: 0, end: 3, head: '가나다', tail: '가나다' }]);
  assert.equal(threads[1].issueIndex, 1);
  assert.equal(threads[1].issueId, null);
  assert.equal(threads[1].pass, 'JUDGMENT');
});

test('threadsFromResult - feedback 결과도 같은 방식으로 편다', () => {
  const outcome = {
    version: 1,
    kind: 'feedback',
    issues: [{ trait: '군더더기', pass: 'stylistic', body: null, anchors: [] }],
    conclusion: { understanding: null, patterns: [], priorities: [] },
  } satisfies ReviewOutcome;

  assert.equal(threadsFromResult(outcome).length, 1);
});

import assert from 'node:assert/strict';
import test from 'node:test';
import dayjs from 'dayjs';
import { ConfirmInputSchema, confirmResult, manuscriptPath, pickVersion, roundState, summarizeOutcome } from './prism-review-core.ts';

test('summarizeOutcome: feedback은 집계, issues는 건수만, rejected는 문면, null은 전부 비움', () => {
  const summarized = summarizeOutcome({
    version: 1,
    kind: 'feedback',
    issues: [{ trait: 't', pass: 'judgment', body: null, anchors: [] }],
    conclusion: {
      understanding: '리드',
      patterns: [{ theme: null, body: 'p', issues: [] }],
      priorities: [],
      strengths: [{ start: 0, end: 1, head: 'h', tail: 't', body: null }],
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
  assert.deepEqual(confirmResult('PRRR1', 'high', { title: '제목', subtitle: null, path: 'manuscript/PRDV1.txt' }), {
    decision: 'confirmed',
    key: 'PRRR1',
    tier: 'high',
    document: { title: '제목', subtitle: null, path: 'manuscript/PRDV1.txt' },
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

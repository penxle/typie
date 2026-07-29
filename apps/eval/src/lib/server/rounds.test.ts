import { describe, expect, it } from 'vitest';
import { planRound } from './rounds.ts';
import type { RoundRun } from './rounds.ts';

const run = (over: Partial<RoundRun> & { id: string }): RoundRun => ({
  status: 'done',
  generationId: 'editorial',
  documentId: `doc-${over.id}`,
  documentKind: 'sampled',
  refId: `ref-${over.id}`,
  ...over,
});

const plan = (runs: RoundRun[], usedDocumentIds: string[] = []) =>
  planRound({
    runs,
    requestedIds: runs.map((r) => r.id),
    generation: { id: 'editorial', label: '에디토리얼' },
    usedDocumentIds,
  });

describe('planRound', () => {
  it('고른 실행이 없으면 거부한다', () => {
    expect(plan([])).toEqual({ error: '평가할 실행이 없습니다' });
  });

  it('찾지 못한 실행을 이름으로 밝힌다', () => {
    const result = planRound({
      runs: [run({ id: 'a' })],
      requestedIds: ['a', 'ghost'],
      generation: { id: 'editorial', label: '에디토리얼' },
      usedDocumentIds: [],
    });
    expect(result).toEqual({ error: '실행을 찾을 수 없습니다: ghost' });
  });

  it('완료되지 않은 실행을 거부한다', () => {
    expect(plan([run({ id: 'a' }), run({ id: 'b', status: 'running' })])).toEqual({
      error: '완료되지 않은 실행이 있습니다: b',
    });
  });

  it('다른 세대의 실행이 섞이면 거부한다', () => {
    expect(plan([run({ id: 'a', generationId: 'analysis' })])).toEqual({
      error: '에디토리얼 평가에 다른 세대의 실행이 섞였습니다: a',
    });
  });

  it('반입 문서를 거부한다', () => {
    expect(plan([run({ id: 'a' }), run({ id: 'b', documentKind: 'intake' })])).toEqual({
      error: '반입 문서는 라운드에 넣을 수 없습니다: ref-b',
    });
  });

  // 표식이 없는 문서는 표집으로 치지 않는다 — 판별이 안 되면 라운드에서 빠지는 쪽이 안전하다.
  it('출처 표식이 없는 문서를 거부한다', () => {
    expect(plan([run({ id: 'a', documentKind: null })])).toEqual({
      error: '반입 문서는 라운드에 넣을 수 없습니다: ref-a',
    });
  });

  it('이미 다른 라운드에 쓰인 문서를 거부한다', () => {
    expect(plan([run({ id: 'a' }), run({ id: 'b' })], ['doc-b'])).toEqual({
      error: '이미 다른 라운드에 쓰인 문서입니다: ref-b',
    });
  });

  // 실행 id가 서로 달라도 같은 원고면 평가자가 두 번 읽는다.
  it('한 라운드 안에서 같은 문서의 실행 둘을 거부한다', () => {
    expect(plan([run({ id: 'a', documentId: 'doc-x' }), run({ id: 'b', documentId: 'doc-x' })])).toEqual({
      error: '같은 문서의 실행이 둘 이상 섞였습니다: ref-b',
    });
  });

  it('refId가 없으면 실행 id로 밝힌다', () => {
    expect(plan([run({ id: 'a', documentKind: 'intake', refId: null })])).toEqual({
      error: '반입 문서는 라운드에 넣을 수 없습니다: a',
    });
  });

  it('아직 쓰이지 않은 표집 문서의 완료 실행은 통과한다', () => {
    expect(plan([run({ id: 'a' }), run({ id: 'b' })], ['doc-other'])).toEqual({ ok: true });
  });
});

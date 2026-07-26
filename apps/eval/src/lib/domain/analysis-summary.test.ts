import { describe, expect, it } from 'vitest';
import { collectRejections, collectReviewNotes, rate, summarizeAnalysis } from './analysis-summary.ts';
import type { FeedbackRef, VerdictRow } from './analysis-summary.ts';

const feedback = (id: string, setId: string, ord: number, category = '대화 화자'): FeedbackRef => ({
  id,
  setId,
  ord,
  category,
  polarity: 'issue',
});

const verdict = (judgmentId: string, feedbackId: string, v: Partial<VerdictRow> = {}): VerdictRow => ({
  judgmentId,
  feedbackId,
  correct: true,
  needed: true,
  useful: true,
  note: null,
  ...v,
});

const taskBySet = new Map([
  ['s1', 'task-1'],
  ['s2', 'task-2'],
]);

const documentBySet = new Map([
  ['s1', { refId: 'DOC1', characterCount: 8000 }],
  ['s2', { refId: 'DOC2', characterCount: 20_000 }],
]);

describe('rate', () => {
  it('답한 것만 분모에 넣는다', () => {
    expect(rate({ yes: 3, no: 1 })).toBe(0.75);
  });

  it('아무도 답하지 않았으면 비율이 없다', () => {
    expect(rate({ yes: 0, no: 0 })).toBeNaN();
  });
});

describe('summarizeAnalysis', () => {
  const feedbacks = [feedback('f1', 's1', 0), feedback('f2', 's1', 1), feedback('f3', 's2', 0)];

  it('축마다 예·아니오를 센다', () => {
    const result = summarizeAnalysis({
      verdicts: [verdict('j1', 'f1'), verdict('j1', 'f2', { correct: false }), verdict('j2', 'f3', { needed: false, useful: false })],
      reviewVerdicts: [],
      helpfulness: [],
      feedbacks,
      documentBySet,
    });
    expect(result.axes.correct).toEqual({ yes: 2, no: 1 });
    expect(result.axes.needed).toEqual({ yes: 2, no: 1 });
    expect(result.axes.useful).toEqual({ yes: 2, no: 1 });
  });

  it('문서를 아니오 비율이 높은 순으로 놓는다', () => {
    const result = summarizeAnalysis({
      verdicts: [verdict('j1', 'f1'), verdict('j1', 'f2'), verdict('j2', 'f3', { correct: false })],
      reviewVerdicts: [],
      helpfulness: [],
      feedbacks,
      documentBySet,
    });
    expect(result.documents.map((d) => d.refId)).toEqual(['DOC2', 'DOC1']);
    expect(result.documents[0]).toMatchObject({ refId: 'DOC2', feedbacks: 1, judged: 1, no: 1 });
    expect(result.documents[1]).toMatchObject({ refId: 'DOC1', feedbacks: 2, judged: 2, no: 0 });
  });

  it('같은 피드백에 답이 둘 이상일 때만 일치도를 센다', () => {
    const result = summarizeAnalysis({
      verdicts: [verdict('j1', 'f1'), verdict('j2', 'f1', { correct: false }), verdict('j1', 'f2')],
      reviewVerdicts: [],
      helpfulness: [],
      feedbacks,
      documentBySet,
    });
    const correct = result.agreement.find((a) => a.axis === 'correct');
    expect(correct).toEqual({ axis: 'correct', pairs: 1, agreed: 0 });
    const needed = result.agreement.find((a) => a.axis === 'needed');
    expect(needed).toEqual({ axis: 'needed', pairs: 1, agreed: 1 });
  });

  it('판정이 하나뿐이면 일치도 표본이 없다', () => {
    const result = summarizeAnalysis({
      verdicts: [verdict('j1', 'f1')],
      reviewVerdicts: [],
      helpfulness: [],
      feedbacks,
      documentBySet,
    });
    expect(result.agreement.every((a) => a.pairs === 0)).toBe(true);
  });

  it('미판정(null)은 어느 쪽에도 세지 않는다', () => {
    const result = summarizeAnalysis({
      verdicts: [verdict('j1', 'f1', { correct: null })],
      reviewVerdicts: [],
      helpfulness: [],
      feedbacks,
      documentBySet,
    });
    expect(result.axes.correct).toEqual({ yes: 0, no: 0 });
    expect(result.axes.needed).toEqual({ yes: 1, no: 0 });
  });
});

describe('collectRejections', () => {
  const detail = (id: string, setId: string, ord: number, body: string, category = '대화 화자') => ({
    id,
    setId,
    ord,
    category,
    polarity: 'issue',
    body,
  });
  const feedbacks = [detail('f1', 's1', 0, '화자가 뒤바뀝니다'), detail('f2', 's1', 4, '같은 표현이 반복됩니다')];
  const at = new Date('2026-07-27T10:00:00Z');

  const rejection = (v: Partial<VerdictRow>, feedbackId = 'f1', evaluator = 'a@x') => ({
    ...verdict('j1', feedbackId, v),
    at,
    evaluator,
  });

  it('판정 대상과 어느 축이 아니오인지를 함께 싣는다', () => {
    const rows = collectRejections({
      verdicts: [rejection({ correct: false, note: '본문에 없는 말입니다' })],
      feedbacks,
      documentBySet,
      taskBySet,
    });
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      number: 1,
      category: '대화 화자',
      body: '화자가 뒤바뀝니다',
      refId: 'DOC1',
      evaluator: 'a@x',
      failed: { correct: true, needed: false, useful: false },
      note: '본문에 없는 말입니다',
    });
  });

  // 사유 없는 '아니오'를 빼면 화면에 보이는 반대가 실제보다 적어 보인다.
  it('사유가 없어도 아니오면 모은다', () => {
    const rows = collectRejections({ verdicts: [rejection({ needed: false })], feedbacks, documentBySet, taskBySet });
    expect(rows).toHaveLength(1);
    expect(rows[0].note).toBeNull();
    expect(rows[0].failed).toEqual({ correct: false, needed: true, useful: false });
  });

  it('공백뿐인 사유는 없는 것으로 본다', () => {
    const rows = collectRejections({ verdicts: [rejection({ useful: false, note: ' '.repeat(3) })], feedbacks, documentBySet, taskBySet });
    expect(rows[0].note).toBeNull();
  });

  it('전부 예면 모으지 않는다', () => {
    expect(collectRejections({ verdicts: [rejection({})], feedbacks, documentBySet, taskBySet })).toEqual([]);
  });

  it('축 여러 개가 아니오면 전부 표시한다', () => {
    const rows = collectRejections({ verdicts: [rejection({ correct: false, useful: false })], feedbacks, documentBySet, taskBySet });
    expect(rows[0].failed).toEqual({ correct: true, needed: false, useful: true });
  });

  // 한 문서의 반대를 모아 읽어야 패턴이 보인다 — 시간순으로 흩으면 문서가 뒤섞인다.
  it('문서별로 묶고 그 안에서는 피드백 번호순으로 놓는다', () => {
    const rows = collectRejections({
      verdicts: [
        rejection({ correct: false }, 'f2'),
        rejection({ correct: false }, 'f1'),
        { ...verdict('j2', 'f3', { correct: false }), at, evaluator: 'b@x' },
      ],
      feedbacks: [...feedbacks, detail('f3', 's2', 0, '다른 문서')],
      documentBySet,
      taskBySet,
    });
    expect(rows.map((r) => [r.refId, r.number])).toEqual([
      ['DOC1', 1],
      ['DOC1', 5],
      ['DOC2', 1],
    ]);
  });

  it('미판정(null)은 아니오가 아니다', () => {
    expect(collectRejections({ verdicts: [rejection({ correct: null })], feedbacks, documentBySet, taskBySet })).toEqual([]);
  });
});

describe('collectReviewNotes', () => {
  const at = new Date('2026-07-27T10:00:00Z');
  const judgments = [{ id: 'j1', evaluator: 'a@x', comment: null, at }];
  const review = (v: Partial<{ readCorrectly: boolean | null; priorityUseful: boolean | null; note: string | null }> = {}) => ({
    judgmentId: 'j1',
    setId: 's1',
    readCorrectly: true,
    priorityUseful: true,
    note: null,
    ...v,
  });

  it('아니오 축과 사유를 문서·평가자와 함께 싣는다', () => {
    const rows = collectReviewNotes({
      reviewVerdicts: [review({ priorityUseful: false, note: '순서에 동의 못 함' })],
      judgments,
      documentBySet,
      taskBySet,
    });
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      refId: 'DOC1',
      taskId: 'task-1',
      evaluator: 'a@x',
      failed: { readCorrectly: false, priorityUseful: true },
      note: '순서에 동의 못 함',
    });
  });

  // 둘 다 예이고 남긴 말도 없으면 읽을 것이 없다 — 목록에 두면 실제 내용이 묻힌다.
  it('전부 예이고 아무 말도 없으면 빼놓는다', () => {
    expect(collectReviewNotes({ reviewVerdicts: [review()], judgments, documentBySet, taskBySet })).toEqual([]);
  });

  it('판정이 전부 예여도 남긴 말이 있으면 싣는다', () => {
    const rows = collectReviewNotes({
      reviewVerdicts: [review()],
      judgments: [{ id: 'j1', evaluator: 'a@x', comment: '전반적으로 좋았습니다', at }],
      documentBySet,
      taskBySet,
    });
    expect(rows).toHaveLength(1);
    expect(rows[0].comment).toBe('전반적으로 좋았습니다');
  });

  it('공백뿐인 사유·코멘트는 없는 것으로 본다', () => {
    const rows = collectReviewNotes({
      reviewVerdicts: [review({ readCorrectly: false, note: ' '.repeat(2) })],
      judgments: [{ id: 'j1', evaluator: 'a@x', comment: ' '.repeat(2), at }],
      documentBySet,
      taskBySet,
    });
    expect(rows[0].note).toBeNull();
    expect(rows[0].comment).toBeNull();
  });

  it('문서·평가자 순으로 놓는다', () => {
    const rows = collectReviewNotes({
      reviewVerdicts: [
        { ...review({ readCorrectly: false }), judgmentId: 'j2', setId: 's2' },
        { ...review({ readCorrectly: false }), judgmentId: 'j1', setId: 's1' },
      ],
      judgments: [...judgments, { id: 'j2', evaluator: 'b@x', comment: null, at }],
      documentBySet,
      taskBySet,
    });
    expect(rows.map((r) => r.refId)).toEqual(['DOC1', 'DOC2']);
  });
});

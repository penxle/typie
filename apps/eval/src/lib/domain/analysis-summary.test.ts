import { describe, expect, it } from 'vitest';
import { collectRejectionNotes, rate, summarizeAnalysis } from './analysis-summary.ts';
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

describe('collectRejectionNotes', () => {
  const feedbacks = [feedback('f1', 's1', 0, '대화 화자'), feedback('f2', 's1', 4, '표현 반복')];

  it("'아니오'에 사유가 달린 것만 모으고 어느 축인지 밝힌다", () => {
    const notes = collectRejectionNotes({
      verdicts: [
        { ...verdict('j1', 'f1', { correct: false, note: '본문에 없는 말입니다' }), at: new Date('2026-07-27T10:00:00Z') },
        { ...verdict('j1', 'f2', { note: '좋았어요' }), at: new Date('2026-07-27T09:00:00Z') },
      ],
      feedbacks,
    });
    expect(notes).toHaveLength(1);
    expect(notes[0]).toMatchObject({ number: 1, category: '대화 화자', axes: ['정확'], note: '본문에 없는 말입니다' });
  });

  it('오래된 것부터 놓는다 — 새 사유가 목록 끝에 붙는다', () => {
    const notes = collectRejectionNotes({
      verdicts: [
        { ...verdict('j1', 'f1', { correct: false, note: '나중' }), at: new Date('2026-07-27T12:00:00Z') },
        { ...verdict('j1', 'f2', { useful: false, note: '먼저' }), at: new Date('2026-07-27T09:00:00Z') },
      ],
      feedbacks,
    });
    expect(notes.map((n) => n.note)).toEqual(['먼저', '나중']);
    expect(notes[1].axes).toEqual(['정확']);
  });
});

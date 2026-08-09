import { describe, expect, it } from 'vitest';
import { answeredAll, askAnswerIndex, buildAnswers, emptyDrafts, isAnswered, toggleChoice, toggleOther } from './questions.ts';
import type { AskAnswer, AskQuestion } from './live.ts';
import type { ReviewQuestionRecord } from './types.ts';

const single: AskQuestion = { question: 'Q1?', hint: 'h', multi: false, options: [{ label: '가' }, { label: '나' }] };
const multi: AskQuestion = { question: 'Q2?', hint: 'h', multi: true, options: [{ label: 'a' }, { label: 'b' }] };

describe('답변 초안', () => {
  it('단일 선택은 라벨 하나만 남고 직접 입력 선택이 풀린다', () => {
    let d = emptyDrafts([single])[0];
    d = toggleChoice(single, d, '가');
    d = toggleChoice(single, d, '나');
    expect(d.choices).toEqual(['나']);
    d = toggleOther(single, d);
    expect(d.choices).toEqual([]);
    expect(d.otherOn).toBe(true);
    d = toggleChoice(single, d, '가');
    expect(d.otherOn).toBe(false);
  });

  it('복수 선택은 라벨과 직접 입력을 함께 담는다', () => {
    let d = emptyDrafts([multi])[0];
    d = toggleChoice(multi, d, 'a');
    d = toggleChoice(multi, d, 'b');
    d = toggleOther(multi, d);
    d = { ...d, other: '대사의 자연스러움' };
    expect(buildAnswers([multi], [d])).toEqual([{ question: 'Q2?', choice: ['a', 'b', '대사의 자연스러움'] }]);
  });

  it('복수 선택은 같은 라벨을 다시 눌러 해제한다', () => {
    let d = emptyDrafts([multi])[0];
    d = toggleChoice(multi, d, 'a');
    d = toggleChoice(multi, d, 'b');
    d = toggleChoice(multi, d, 'a');
    expect(d.choices).toEqual(['b']);
    d = toggleOther(multi, d);
    d = toggleOther(multi, d);
    expect(d.otherOn).toBe(false);
    expect(d.choices).toEqual(['b']);
  });

  it('답변 판정 — 라벨이나 채워진 직접 입력이 있어야 한다', () => {
    const d = emptyDrafts([single])[0];
    expect(isAnswered(d)).toBe(false);
    expect(isAnswered({ ...d, otherOn: true, other: '  ' })).toBe(false);
    expect(isAnswered({ ...d, otherOn: true, other: '답' })).toBe(true);
    expect(isAnswered({ ...d, choices: ['가'] })).toBe(true);
  });

  it('제출 판정은 질문 하나라도 비면 막는다', () => {
    const drafts = emptyDrafts([single, multi]);
    expect(answeredAll(drafts)).toBe(false);
    expect(answeredAll([toggleChoice(single, drafts[0], '가'), drafts[1]])).toBe(false);
    expect(answeredAll([toggleChoice(single, drafts[0], '가'), toggleChoice(multi, drafts[1], 'a')])).toBe(true);
  });

  it('제출 답변은 사영 기록이 되읽는 모양 그대로다', () => {
    const answers: AskAnswer[] = buildAnswers([single], [toggleChoice(single, emptyDrafts([single])[0], '가')]);
    expect(answers).toEqual([{ question: 'Q1?', choice: ['가'] }]);
  });

  it('빈 직접 입력은 choice에 실리지 않는다', () => {
    let d = emptyDrafts([single])[0];
    d = toggleChoice(single, d, '가');
    expect(buildAnswers([single], [d])).toEqual([{ question: 'Q1?', choice: ['가'] }]);
  });

  it('직접 입력은 앞뒤 공백을 떼고 실린다', () => {
    let d = emptyDrafts([single])[0];
    d = toggleOther(single, d);
    d = { ...d, other: '  손으로 쓴 답  ' };
    expect(buildAnswers([single], [d])).toEqual([{ question: 'Q1?', choice: ['손으로 쓴 답'] }]);
  });

  it('여러 질문의 초안은 서로 독립이고 답도 질문 순서로 실린다', () => {
    const drafts = emptyDrafts([single, multi]);
    expect(drafts).toHaveLength(2);
    const next = [toggleChoice(single, drafts[0], '가'), toggleChoice(multi, drafts[1], 'b')];
    expect(drafts[0].choices).toEqual([]);
    expect(buildAnswers([single, multi], next)).toEqual([
      { question: 'Q1?', choice: ['가'] },
      { question: 'Q2?', choice: ['b'] },
    ]);
  });
});

describe('사영 기록의 답변 색인', () => {
  const record = (over: Partial<ReviewQuestionRecord>): ReviewQuestionRecord => ({
    agentName: 'plan',
    toolCallId: 'call_1',
    stage: 'plan',
    at: 1000,
    status: 'answered',
    questions: [single],
    answers: [{ question: 'Q1?', choice: ['가'] }],
    ...over,
  });

  it('기록이 없으면 빈 색인이다', () => {
    expect(askAnswerIndex(null)).toEqual({});
    expect(askAnswerIndex([])).toEqual({});
  });

  it('답변을 toolCallId로 색인하고 답 없는 질문은 넣지 않는다', () => {
    const records = [record({}), record({ toolCallId: 'call_2', status: 'closed', answers: null })];
    expect(askAnswerIndex(records)).toEqual({ call_1: [{ question: 'Q1?', choice: ['가'] }] });
  });
});

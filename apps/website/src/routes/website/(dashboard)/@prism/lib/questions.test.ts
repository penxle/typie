import { describe, expect, it } from 'vitest';
import { answeredAll, buildAnswers, emptyDrafts, isAnswered, toggleChoice, toggleOther } from './questions.ts';
import type { AskQuestion } from '@typie/prism';

const single: AskQuestion = { question: '질문 하나?', hint: 'h', multi: false, options: [{ label: '가' }, { label: '나' }] };
const multi: AskQuestion = { question: '질문 둘?', hint: 'h', multi: true, options: [{ label: 'a' }, { label: 'b' }] };

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
    expect(buildAnswers([multi], [d])).toEqual([{ question: '질문 둘?', choice: ['a', 'b', '대사의 자연스러움'] }]);
  });

  it('복수 선택은 같은 라벨을 다시 눌러 해제한다', () => {
    let d = emptyDrafts([multi])[0];
    d = toggleChoice(multi, d, 'a');
    d = toggleChoice(multi, d, 'b');
    d = toggleChoice(multi, d, 'a');
    expect(d.choices).toEqual(['b']);
  });

  it('빈 직접 입력은 답변이 아니고, 전부 답해야 answeredAll', () => {
    const drafts = emptyDrafts([single, multi]);
    expect(isAnswered({ ...drafts[0], otherOn: true, other: '  ' })).toBe(false);
    expect(answeredAll([toggleChoice(single, drafts[0], '가'), drafts[1]])).toBe(false);
    expect(answeredAll([toggleChoice(single, drafts[0], '가'), toggleChoice(multi, drafts[1], 'a')])).toBe(true);
  });
});

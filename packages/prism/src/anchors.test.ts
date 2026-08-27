import { describe, expect, it } from 'vitest';
import { anchorQuote } from './anchors.ts';
import type { ResolvedAnchor } from './anchors.ts';

const resolved = (text: string, head = '', tail = ''): ResolvedAnchor => ({ head, tail, selection: null, text });

describe('anchorQuote', () => {
  it('앵커의 원문을 줄임표로 잇는다', () => {
    expect(anchorQuote([resolved('가나'), resolved('마바사')])).toBe('가나 ⋯ 마바사');
  });

  it('head·tail보다 긴 원문은 머리·꼬리만 남기고 중간을 줄임표로 접는다', () => {
    expect(anchorQuote([resolved(`머리글 ${'가'.repeat(200)} 꼬리글`, '머리글', '꼬리글')])).toBe('머리글 ⋯ 꼬리글');
  });

  it('head·tail이 구간을 다 덮는 짧은 앵커는 원문 그대로 쓴다', () => {
    expect(anchorQuote([resolved('가나다라마', '가나다', '다라마')])).toBe('가나다라마');
  });

  it('head나 tail이 비면 접지 않고 원문을 그대로 쓴다', () => {
    expect(anchorQuote([resolved('가나다라마바사')])).toBe('가나다라마바사');
  });

  it('원문이 없는 앵커(자리 없음)는 머리·꼬리로 서고, 머리와 꼬리가 같으면 한 번만 쓴다', () => {
    expect(anchorQuote([{ head: '머리', tail: '꼬리', selection: null, text: null }])).toBe('머리 ⋯ 꼬리');
    expect(anchorQuote([{ head: '같다', tail: '같다', selection: null, text: null }])).toBe('같다');
    expect(anchorQuote([{ head: '머리만', tail: '', selection: null, text: null }])).toBe('머리만');
  });

  it('빈 앵커는 버린다', () => {
    expect(anchorQuote([])).toBe('');
    expect(anchorQuote([{ head: '', tail: '', selection: null, text: null }])).toBe('');
    expect(anchorQuote([resolved('  ')])).toBe('');
  });
});

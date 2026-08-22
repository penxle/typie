import { describe, expect, it } from 'vitest';
import { anchorQuote } from './anchors.ts';

describe('anchorQuote', () => {
  it('원문 구간을 잘라 줄임표로 잇는다', () => {
    const quote = anchorQuote('가나다라마바사', [
      { start: 0, end: 2, head: '', tail: '' },
      { start: 4, end: 7, head: '', tail: '' },
    ]);
    expect(quote).toBe('가나 ⋯ 마바사');
  });

  it('범위 밖·역전 앵커는 버린다', () => {
    expect(anchorQuote('가나다', [{ start: 2, end: 9, head: '', tail: '' }])).toBe('');
    expect(anchorQuote('가나다', [])).toBe('');
  });

  it('head·tail보다 긴 구간은 머리·꼬리만 남기고 중간을 줄임표로 접는다', () => {
    const content = `머리글 ${'가'.repeat(200)} 꼬리글`;
    const quote = anchorQuote(content, [{ start: 0, end: content.length, head: '머리글', tail: '꼬리글' }]);
    expect(quote).toBe('머리글 ⋯ 꼬리글');
  });

  it('head·tail이 구간을 다 덮는 짧은 앵커는 원문 그대로 쓴다', () => {
    expect(anchorQuote('가나다라마', [{ start: 0, end: 5, head: '가나다', tail: '다라마' }])).toBe('가나다라마');
  });

  it('head나 tail이 비면 접지 않고 원문 구간을 그대로 쓴다', () => {
    expect(anchorQuote('가나다라마바사', [{ start: 0, end: 7, head: '', tail: '' }])).toBe('가나다라마바사');
  });
});

import { describe, expect, it } from 'vitest';
import { anchorPosition, anchorQuote, markParagraphs, markSegments } from './anchors.ts';

const thread = (id: string, start: number, end: number, pass: 'critique' | 'proofread', state = 'open') =>
  ({ id, pass, state, anchors: [{ start, end, head: '', tail: '' }] }) as never;

describe('markSegments', () => {
  it('스레드 소속을 세그먼트로 분해한다 — 앵커 길이와 무관하게 같은 형상이다', () => {
    const segs = markSegments('가나다라마바사', [thread('t1', 1, 3, 'critique'), thread('t2', 2, 5, 'proofread', 'closed')]);
    expect(segs.map((s) => s.text).join('')).toBe('가나다라마바사');
    const hit = segs.find((s) => s.text === '다');
    expect(hit?.threadIds).toEqual(['t1', 't2']);
  });

  it('앵커 없는 스레드는 소속을 만들지 않는다', () => {
    const segs = markSegments('가나다', [{ id: 't1', pass: 'critique', state: 'open', anchors: [] } as never]);
    expect(segs).toEqual([{ text: '가나다', threadIds: [] }]);
  });

  it('긴 앵커와 짧은 앵커가 겹친 조각은 양쪽 소속을 모두 싣는다', () => {
    const content = `머리 ${'가'.repeat(50)} 꼬리`;
    const segs = markSegments(content, [
      { id: 'long', pass: 'critique', state: 'open', anchors: [{ start: 0, end: content.length, head: '머리', tail: '꼬리' }] } as never,
      thread('short', 5, 9, 'proofread'),
    ]);
    const hit = segs.find((s) => s.text === '가가가가');
    expect(hit?.threadIds).toEqual(['long', 'short']);
  });

  it('한 스레드의 겹치는 앵커가 같은 세그먼트를 덮어도 한 번만 센다', () => {
    const segs = markSegments('가나다라마', [
      {
        id: 't1',
        pass: 'proofread',
        state: 'open',
        anchors: [
          { start: 0, end: 4, head: '', tail: '' },
          { start: 1, end: 3, head: '', tail: '' },
        ],
      } as never,
    ]);
    const hit = segs.find((s) => s.text === '나다');
    expect(hit?.threadIds).toEqual(['t1']);
  });
});

describe('markParagraphs', () => {
  it('개행으로 문단을 가르고 빈 문단을 버린다', () => {
    const paragraphs = markParagraphs('가나\n\n다라', [thread('t1', 1, 5, 'critique')]);
    expect(paragraphs.map((pieces) => pieces.map((piece) => piece.text).join(''))).toEqual(['가나', '다라']);
    expect(paragraphs[0].map((piece) => piece.threadIds)).toEqual([[], ['t1']]);
    expect(paragraphs[1].map((piece) => piece.threadIds)).toEqual([['t1'], []]);
  });

  it('문단 양끝 공백을 다듬는다', () => {
    const paragraphs = markParagraphs('  가나  \n다', []);
    expect(paragraphs.map((pieces) => pieces.map((piece) => piece.text).join(''))).toEqual(['가나', '다']);
  });
});

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

describe('anchorPosition', () => {
  it('첫 앵커 시작을 반올림해 표기한다', () => {
    expect(anchorPosition([{ start: 4812, end: 4900, head: '', tail: '' }])).toBe('4,800자 부근');
    expect(anchorPosition([])).toBe('위치 없음');
  });

  it('무효한 첫 앵커는 건너뛰고 인용과 같은 앵커를 가리킨다', () => {
    expect(
      anchorPosition([
        { start: 900, end: 100, head: '', tail: '' },
        { start: 4812, end: 4900, head: '', tail: '' },
      ]),
    ).toBe('4,800자 부근');
    expect(anchorPosition([{ start: -1, end: 100, head: '', tail: '' }])).toBe('위치 없음');
  });
});

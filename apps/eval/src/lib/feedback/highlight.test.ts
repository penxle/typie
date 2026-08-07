import { describe, expect, it } from 'vitest';
import { computeSegments } from './highlight.ts';

describe('computeSegments', () => {
  it('앵커 없는 텍스트는 단일 세그먼트', () => {
    expect(computeSegments('hello', [])).toEqual([{ text: 'hello', feedbackIds: [] }]);
  });

  it('단일 앵커를 분할한다', () => {
    const segments = computeSegments('abcdef', [{ start: 2, end: 4, feedbackId: 'f1' }]);
    expect(segments).toEqual([
      { text: 'ab', feedbackIds: [] },
      { text: 'cd', feedbackIds: ['f1'] },
      { text: 'ef', feedbackIds: [] },
    ]);
  });

  it('겹치는 앵커는 feedbackIds가 합쳐진다', () => {
    const segments = computeSegments('abcdef', [
      { start: 0, end: 4, feedbackId: 'f1' },
      { start: 2, end: 6, feedbackId: 'f2' },
    ]);
    expect(segments).toEqual([
      { text: 'ab', feedbackIds: ['f1'] },
      { text: 'cd', feedbackIds: ['f1', 'f2'] },
      { text: 'ef', feedbackIds: ['f2'] },
    ]);
  });

  it('역전·범위 밖 앵커는 무시한다', () => {
    const segments = computeSegments('abc', [
      { start: 2, end: 1, feedbackId: 'bad1' },
      { start: 10, end: 12, feedbackId: 'bad2' },
    ]);
    expect(segments).toEqual([{ text: 'abc', feedbackIds: [] }]);
  });
});

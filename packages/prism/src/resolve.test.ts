import { describe, expect, it } from 'vitest';
import { resolveAnchors } from './resolve.ts';

describe('resolveAnchors', () => {
  it('머리와 꼬리를 원고에서 찾아 구간을 연다', () => {
    const text = '가나다 라마바 사아자';
    expect(resolveAnchors(text, [{ head: '가나다', tail: '사아자' }])).toEqual([{ start: 0, end: text.length }]);
  });

  it('같은 구절이 여럿이면 앞에서부터 첫 자리를 고른다 — prism이 검증한 자리와 같은 규칙', () => {
    const text = '반복 구절\n\n사이 글\n\n반복 구절';
    expect(resolveAnchors(text, [{ head: '반복 구절', tail: '반복 구절' }])).toEqual([{ start: 0, end: 5 }]);
  });

  it('꼬리가 머리 인용 범위 안의 문장이어도 유효하다', () => {
    const text = '가나다 라마바';
    expect(resolveAnchors(text, [{ head: '가나다 라마바', tail: '라마바' }])).toEqual([{ start: 0, end: text.length }]);
  });

  it('꼬리가 머리보다 앞에 있으면 둘을 이어 붙인다', () => {
    const text = '꼬리 구절\n\n사이 글\n\n머리 구절';
    expect(resolveAnchors(text, [{ head: '머리 구절', tail: '꼬리 구절' }])).toEqual([{ start: 0, end: text.length }]);
  });

  it('가나 잡음과 덧붙은 문장부호는 걷어내고 다시 찾는다', () => {
    expect(resolveAnchors('가나다 라마바', [{ head: 'エ가나다', tail: '라마바…' }])).toEqual([{ start: 0, end: 7 }]);
  });

  it('공백이 달라져도 찾는다', () => {
    const text = '가나다\n라마바';
    expect(resolveAnchors(text, [{ head: '가나다 라마바', tail: '라마바' }])).toEqual([{ start: 0, end: text.length }]);
  });

  it('따옴표가 달라져도 찾는다', () => {
    const text = '그가 “가나다”라고 했다';
    expect(resolveAnchors(text, [{ head: '"가나다"', tail: '했다' }])).toEqual([{ start: text.indexOf('가나다'), end: text.length }]);
  });

  it('느슨한 일치보다 정확한 일치를 앞세운다', () => {
    const text = '그가 “가나다”라고 했다\n\n그가 "가나다"라고 했다';
    expect(resolveAnchors(text, [{ head: '"가나다"', tail: '했다' }])).toEqual([{ start: text.indexOf('"가나다"'), end: text.length }]);
  });

  it('꼬리를 찾지 못하면 실패한다', () => {
    expect(resolveAnchors('가나다 사라짐', [{ head: '가나다', tail: '라마바' }])).toEqual([null]);
  });

  it('머리와 꼬리가 둘 다 비면 찾을 근거가 없으므로 실패한다', () => {
    expect(resolveAnchors('가나다라마바', [{ head: '', tail: '' }])).toEqual([null]);
    expect(resolveAnchors('가나', [{ head: '', tail: '' }])).toEqual([null]);
  });

  it('머리가 수없이 반복되고 꼬리가 사라져도 곧 포기한다', () => {
    const text = '가나다라마바사아자차'.repeat(20_000);
    const anchors = [
      { head: '가', tail: '사라진 꼬리' },
      { head: '가', tail: '사라진 꼬리' },
    ];
    const started = performance.now();
    expect(resolveAnchors(text, anchors)).toEqual([null, null]);
    expect(performance.now() - started).toBeLessThan(1500);
  });

  it('앵커마다 독립으로 판정한다', () => {
    const text = '가나다 라마바';
    expect(
      resolveAnchors(text, [
        { head: '가나다', tail: '라마바' },
        { head: '없는말', tail: '없는말' },
      ]),
    ).toEqual([{ start: 0, end: text.length }, null]);
  });
});

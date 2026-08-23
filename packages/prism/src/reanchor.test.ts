import { describe, expect, it } from 'vitest';
import { reanchorAll } from './reanchor.ts';
import type { Anchor } from './review.ts';

const anchor = (text: string, head: string, tail: string): Anchor => {
  const start = text.indexOf(head);
  const end = text.lastIndexOf(tail) + tail.length;
  return { start, end, head, tail };
};

describe('reanchorAll', () => {
  it('원고가 그대로면 원래 좌표를 돌려준다', () => {
    const text = '가나다 라마바 사아자';
    const a = anchor(text, '가나다', '사아자');
    expect(reanchorAll(text, [a])).toEqual([{ start: a.start, end: a.end }]);
  });

  it('앞에 글이 삽입되면 삽입된 길이만큼 밀린다', () => {
    const before = '가나다 라마바';
    const a = anchor(before, '가나다', '라마바');
    const prefix = '머리말 한 줄\n\n';
    const after = `${prefix}${before}`;
    expect(reanchorAll(after, [a])).toEqual([{ start: prefix.length, end: prefix.length + before.length }]);
  });

  it('같은 구절이 여럿이면 원래 좌표에 가장 가까운 것을 고른다', () => {
    const text = '반복 구절\n\n사이 글\n\n반복 구절\n\n뒤 글\n\n반복 구절';
    const third = text.lastIndexOf('반복 구절');
    const a: Anchor = { start: third, end: third + 5, head: '반복 구절', tail: '반복 구절' };
    expect(reanchorAll(text, [a])).toEqual([{ start: third, end: third + 5 }]);
  });

  it('앞쪽에 같은 머리가 잔뜩 있어도 첫 자리가 아니라 원래 좌표의 자리를 고른다', () => {
    const unit = '반복 구절\n\n';
    const text = unit.repeat(40);
    const late = unit.length * 35;
    const a: Anchor = { start: late, end: late + 5, head: '반복 구절', tail: '반복 구절' };
    expect(reanchorAll(text, [a])).toEqual([{ start: late, end: late + 5 }]);
  });

  it('꼬리가 머리보다 앞에 있으면 둘을 이어 붙인다', () => {
    const text = '꼬리 구절\n\n사이 글\n\n머리 구절';
    const a: Anchor = { start: 0, end: text.length, head: '머리 구절', tail: '꼬리 구절' };
    expect(reanchorAll(text, [a])).toEqual([{ start: 0, end: text.length }]);
  });

  it('가나 잡음과 덧붙은 문장부호는 걷어내고 다시 찾는다', () => {
    const text = '가나다 라마바';
    const a: Anchor = { start: 0, end: 7, head: 'エ가나다', tail: '라마바…' };
    expect(reanchorAll(text, [a])).toEqual([{ start: 0, end: 7 }]);
  });

  it('가까운 느슨한 일치보다 멀더라도 정확한 일치를 앞세운다', () => {
    const near = '그가 “가나다”라고 했다';
    const text = `${near}\n\n그가 "가나다"라고 했다`;
    const a: Anchor = { start: 3, end: near.length, head: '"가나다"', tail: '했다' };
    expect(reanchorAll(text, [a])).toEqual([{ start: text.indexOf('"가나다"'), end: text.length }]);
  });

  it('꼬리를 찾지 못하면 실패한다', () => {
    const before = '가나다 라마바';
    const a = anchor(before, '가나다', '라마바');
    expect(reanchorAll('가나다 사라짐', [a])).toEqual([null]);
  });

  it('공백이 달라져도 찾는다', () => {
    const before = '가나다 라마바';
    const a = anchor(before, '가나다 라마바', '라마바');
    const after = '가나다\n라마바';
    expect(reanchorAll(after, [a])).toEqual([{ start: 0, end: after.length }]);
  });

  it('따옴표가 달라져도 찾는다', () => {
    const before = '그가 "가나다"라고 했다';
    const a = anchor(before, '"가나다"', '했다');
    const after = '그가 “가나다”라고 했다';
    expect(reanchorAll(after, [a])).toEqual([{ start: after.indexOf('가나다'), end: after.length }]);
  });

  it('머리와 꼬리가 둘 다 비면 찾을 근거가 없으므로 실패한다', () => {
    const empty: Anchor = { start: 2, end: 5, head: '', tail: '' };
    expect(reanchorAll('가나다라마바', [empty])).toEqual([null]);
    expect(reanchorAll('가나', [empty])).toEqual([null]);
  });

  it('앞뒤 후보가 똑같이 멀면 뒤쪽을 취한다', () => {
    const unit = '반복 구절';
    const text = unit + '가'.repeat(11) + unit;
    const ahead = text.lastIndexOf(unit);
    // start를 두 후보의 정확한 중간에 둔다 — 동점 규칙이 뒤집히면 뒤 후보가 뽑힌다
    const a: Anchor = { start: ahead / 2, end: ahead / 2 + unit.length, head: unit, tail: unit };
    expect(reanchorAll(text, [a])).toEqual([{ start: 0, end: unit.length }]);
  });

  it('후보가 원래 좌표보다 앞에만 있으면 뒤로 걸어가 찾는다', () => {
    const text = '머리 구절 그리고 꼬리 구절\n\n' + '다른 글\n\n'.repeat(6);
    const head = '머리 구절';
    const tail = '꼬리 구절';
    // 유일한 후보가 start보다 한참 앞이라 앞으로 걷는 경로로는 닿지 않는다
    const a: Anchor = { start: text.length - 10, end: text.length, head, tail };
    expect(reanchorAll(text, [a])).toEqual([{ start: text.indexOf(head), end: text.indexOf(tail) + tail.length }]);
  });

  it('머리가 수없이 반복되고 꼬리가 사라져도 곧 포기한다', () => {
    const text = '가나다라마바사아자차'.repeat(20_000);
    const anchors: Anchor[] = [
      { start: 50_000, end: 50_010, head: '가', tail: '사라진 꼬리' },
      { start: 150_000, end: 150_010, head: '가', tail: '사라진 꼬리' },
    ];
    const started = performance.now();
    expect(reanchorAll(text, anchors)).toEqual([null, null]);
    expect(performance.now() - started).toBeLessThan(1500);
  });

  it('앵커마다 독립으로 판정한다', () => {
    const text = '가나다 라마바';
    const ok = anchor(text, '가나다', '라마바');
    const gone: Anchor = { start: 0, end: 3, head: '없는말', tail: '없는말' };
    expect(reanchorAll(text, [ok, gone])).toEqual([{ start: 0, end: text.length }, null]);
  });
});

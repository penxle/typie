import { describe, expect, it } from 'vitest';
import { diffLines } from './line-diff.ts';

describe('diffLines', () => {
  it('중간 줄이 바뀌면 del 다음 add로 표시한다', () => {
    expect(diffLines('a\nb\nc', 'a\nx\nc')).toEqual([
      { type: 'same', line: 'a' },
      { type: 'del', line: 'b' },
      { type: 'add', line: 'x' },
      { type: 'same', line: 'c' },
    ]);
  });

  it('둘 다 빈 문자열이면 빈 배열', () => {
    expect(diffLines('', '')).toEqual([]);
  });

  it('뒤에 줄이 추가되면 add만 붙는다', () => {
    expect(diffLines('a', 'a\nb')).toEqual([
      { type: 'same', line: 'a' },
      { type: 'add', line: 'b' },
    ]);
  });

  it('완전히 동일하면 전부 same', () => {
    expect(diffLines('a\nb', 'a\nb')).toEqual([
      { type: 'same', line: 'a' },
      { type: 'same', line: 'b' },
    ]);
  });

  it('한쪽이 비어있으면 나머지는 전부 add', () => {
    expect(diffLines('', 'a\nb')).toEqual([
      { type: 'add', line: 'a' },
      { type: 'add', line: 'b' },
    ]);
  });

  it('한쪽이 비어있으면 나머지는 전부 del', () => {
    expect(diffLines('a\nb', '')).toEqual([
      { type: 'del', line: 'a' },
      { type: 'del', line: 'b' },
    ]);
  });
});

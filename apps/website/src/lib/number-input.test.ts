import { describe, expect, test } from 'vitest';
import { formatCommaInput, parseCommaInput } from './number-input';

const type = (value: string, caret?: number) => {
  const el = document.createElement('input');
  el.value = value;
  el.setSelectionRange(caret ?? value.length, caret ?? value.length);
  const formatted = formatCommaInput({ target: el } as unknown as Event);
  return { formatted, caret: el.selectionStart };
};

describe('formatCommaInput', () => {
  test('천 단위 콤마 삽입', () => {
    expect(type('50000').formatted).toBe('50,000');
  });

  test('숫자 외 문자 제거', () => {
    expect(type('12a34').formatted).toBe('1,234');
  });

  test('빈 값 유지', () => {
    expect(type('').formatted).toBe('');
  });

  test('맨끝 타이핑 시 캐럿이 끝에 유지', () => {
    const { formatted, caret } = type('1234');
    expect(formatted).toBe('1,234');
    expect(caret).toBe(5);
  });

  test('중간 편집 시 캐럿 자릿수 보존', () => {
    const { formatted, caret } = type('15,000', 2);
    expect(formatted).toBe('15,000');
    expect(caret).toBe(2);
  });
});

describe('parseCommaInput', () => {
  test('콤마 제거 후 숫자화', () => {
    expect(parseCommaInput('50,000')).toBe(50_000);
  });

  test('빈 문자열은 0', () => {
    expect(parseCommaInput('')).toBe(0);
  });
});

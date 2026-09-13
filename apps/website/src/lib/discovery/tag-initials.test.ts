import { describe, expect, it } from 'vitest';
import { groupTagsByInitial, tagInitial } from './tag-initials.ts';

const range = (start: number, end: number) => Array.from({ length: end - start + 1 }, (_, i) => String.fromCodePoint(start + i));

describe('tagInitial', () => {
  it('한글 음절은 초성으로, 된소리는 예사소리 묶음으로', () => {
    expect(tagInitial('가을')).toBe('ㄱ');
    expect(tagInitial('꽃')).toBe('ㄱ');
    expect(tagInitial('떡')).toBe('ㄷ');
    expect(tagInitial('빵')).toBe('ㅂ');
    expect(tagInitial('쌀')).toBe('ㅅ');
    expect(tagInitial('짜장')).toBe('ㅈ');
    expect(tagInitial('힙합')).toBe('ㅎ');
  });

  it('첫 음절과 끝 음절 경계', () => {
    expect(tagInitial('가')).toBe('ㄱ');
    expect(tagInitial('힣')).toBe('ㅎ');
  });

  it('풀어 쓴(NFD) 음절도 초성으로', () => {
    expect(tagInitial('가을'.normalize('NFD'))).toBe('ㄱ');
  });

  it('초성 자모 19자는 각 초성 묶음으로', () => {
    expect(range(0x11_00, 0x11_12).map(tagInitial).join('')).toBe('ㄱㄱㄴㄷㄷㄹㅁㅂㅂㅅㅅㅇㅈㅈㅊㅋㅌㅍㅎ');
  });

  it('받침 자모 27자는 첫 자음 묶음으로', () => {
    expect(range(0x11_a8, 0x11_c2).map(tagInitial).join('')).toBe('ㄱㄱㄱㄴㄴㄴㄷㄹㄹㄹㄹㄹㄹㄹㄹㅁㅂㅂㅅㅅㅇㅈㅊㅋㅌㅍㅎ');
    expect(tagInitial('ᇂᆻᆸ')).toBe('ㅎ');
  });

  it('호환 자음 30자(ㄱ~ㅎ)는 첫 자음 묶음으로', () => {
    expect(range(0x31_31, 0x31_4e).map(tagInitial).join('')).toBe('ㄱㄱㄱㄴㄴㄴㄷㄷㄹㄹㄹㄹㄹㄹㄹㄹㅁㅂㅂㅂㅅㅅㅇㅈㅈㅊㅋㅌㅍㅎ');
  });

  it('반각 한글 자음 30자도 같은 묶음으로', () => {
    expect(range(0xff_a1, 0xff_be).map(tagInitial).join('')).toBe('ㄱㄱㄱㄴㄴㄴㄷㄷㄹㄹㄹㄹㄹㄹㄹㄹㅁㅂㅂㅂㅅㅅㅇㅈㅈㅊㅋㅌㅍㅎ');
  });

  it('영문은 전각·악센트 포함 A-Z 묶음', () => {
    expect(tagInitial('AI')).toBe('A-Z');
    expect(tagInitial('sf')).toBe('A-Z');
    expect(tagInitial('Ｓｗｉｆｔ')).toBe('A-Z');
    expect(tagInitial('Émile')).toBe('A-Z');
  });

  it('숫자·기호·모음 자모·다른 문자·빈 이름은 기타 묶음', () => {
    expect(tagInitial('3월')).toBe('기타');
    expect(tagInitial('#태그')).toBe('기타');
    expect(tagInitial('ㅏ')).toBe('기타');
    expect(tagInitial('ᅡ')).toBe('기타');
    expect(tagInitial('日記')).toBe('기타');
    expect(tagInitial('한'.normalize('NFD').slice(1))).toBe('기타');
    expect(tagInitial('')).toBe('기타');
  });
});

describe('groupTagsByInitial', () => {
  it('초성 순서 → A-Z → 기타로 묶고 묶음 안은 이름순, 빈 묶음은 뺀다', () => {
    const tags = [
      { name: '소설', count: 3 },
      { name: '2026', count: 4 },
      { name: 'SF', count: 1 },
      { name: '가을', count: 2 },
      { name: 'ᇂᆻᆸ', count: 1 },
      { name: '산책', count: 9 },
      { name: '겨울', count: 1 },
    ];
    expect(groupTagsByInitial(tags)).toEqual([
      {
        initial: 'ㄱ',
        tags: [
          { name: '가을', count: 2 },
          { name: '겨울', count: 1 },
        ],
      },
      {
        initial: 'ㅅ',
        tags: [
          { name: '산책', count: 9 },
          { name: '소설', count: 3 },
        ],
      },
      { initial: 'ㅎ', tags: [{ name: 'ᇂᆻᆸ', count: 1 }] },
      { initial: 'A-Z', tags: [{ name: 'SF', count: 1 }] },
      { initial: '기타', tags: [{ name: '2026', count: 4 }] },
    ]);
  });
});

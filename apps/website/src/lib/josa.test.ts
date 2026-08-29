import { describe, expect, it } from 'vitest';
import { josa } from './josa';

describe('josa', () => {
  it('picks the final-consonant form for a word ending with one', () => {
    expect(josa('구분선', '이', '가')).toBe('이');
    expect(josa('1개의 구분선', '이', '가')).toBe('이');
    expect(josa('3개의 하위 구분선', '이', '가')).toBe('이');
  });

  it('picks the no-final-consonant form for a word ending without one', () => {
    expect(josa('문서', '이', '가')).toBe('가');
    expect(josa('폴더', '이', '가')).toBe('가');
    expect(josa('2개의 문서', '이', '가')).toBe('가');
    expect(josa('5개의 하위 폴더', '이', '가')).toBe('가');
  });

  it('treats the first syllable of the Hangul block as having no final consonant', () => {
    expect(josa('가', '이', '가')).toBe('가');
  });

  it('treats the last syllable of the Hangul block as having a final consonant', () => {
    expect(josa('힣', '이', '가')).toBe('이');
  });

  it('picks the no-final-consonant form for a word ending with a digit', () => {
    expect(josa('7', '이', '가')).toBe('가');
    expect(josa('항목 12', '이', '가')).toBe('가');
  });

  it('picks the no-final-consonant form for a word ending with a latin letter', () => {
    expect(josa('divider', '이', '가')).toBe('가');
    expect(josa('구분선 A', '이', '가')).toBe('가');
  });

  it('returns the no-final-consonant form for an empty string without throwing', () => {
    expect(josa('', '이', '가')).toBe('가');
  });

  it('works for particle pairs other than 이/가', () => {
    expect(josa('구분선', '을', '를')).toBe('을');
    expect(josa('문서', '을', '를')).toBe('를');
  });
});

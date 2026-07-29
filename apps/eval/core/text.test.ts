import { describe, expect, it } from 'vitest';
import { createFindRange, fuzzyFindMatch } from './text.ts';

describe('fuzzyFindMatch', () => {
  it('공백 차이를 흡수한다', () => {
    const match = fuzzyFindMatch('안녕  하세요 반갑습니다', '안녕 하세요', 0);
    expect(match).toEqual({ index: 0, length: 7 });
  });

  it('빈 needle은 null', () => {
    expect(fuzzyFindMatch('abc', '  ', 0)).toBeNull();
  });
});

describe('createFindRange', () => {
  const text = '첫 문장이다. 둘째 문장이다. 셋째 문장이다.';
  const findRange = createFindRange(text);

  it('정확 일치 범위를 찾는다', () => {
    const range = findRange('둘째', '문장이다.', 0);
    expect(range).toEqual({ rangeStart: 8, rangeEnd: 16 });
  });

  it('start=end면 같은 위치를 허용한다', () => {
    const range = findRange('둘째', '둘째', 0);
    expect(range).toEqual({ rangeStart: 8, rangeEnd: 10 });
  });

  it('못 찾으면 null', () => {
    expect(findRange('없는문장', '없는문장', 0)).toBeNull();
  });

  it('end가 start 인용의 뒷문장이면(겹침) 범위를 찾는다', () => {
    const range = findRange('둘째 문장이다. 셋째 문장이다.', '셋째 문장이다.', 0);
    expect(range).toEqual({ rangeStart: 8, rangeEnd: 25 });
  });

  it('end가 start를 포함해도(같은 시작) 범위를 찾는다', () => {
    const range = findRange('둘째 문장이다.', '둘째 문장이다. 셋째 문장이다.', 0);
    expect(range).toEqual({ rangeStart: 8, rangeEnd: 25 });
  });

  it('지문에 따옴표를 날조한 앵커를 정규화로 구제한다', () => {
    const doc = '고양이가 창밖을 본다. 오늘은 나가야겠다.\n그는 문을 열었다.';
    const range = createFindRange(doc)('"오늘은 나가야겠다."', '"오늘은 나가야겠다."', 0);
    expect(range).toEqual({ rangeStart: 13, rangeEnd: 23 });
  });

  it('둥근 따옴표를 곧은 따옴표로 바꾼 앵커를 구제한다', () => {
    const doc = '그가 물었다.\n\n“같이 갈래?”\n\n“좋아.”';
    const range = createFindRange(doc)('"같이 갈래?"', '"좋아."', 0);
    expect(range).toEqual({ rangeStart: 10, rangeEnd: 23 });
  });

  it('공백이 소실된 앵커를 구제한다', () => {
    const doc = '나는 어제 철수 형이 남긴 말을 떠올렸다. 다음 문장.';
    const range = createFindRange(doc)('철수 형이남긴 말을 떠올렸다.', '철수 형이남긴 말을 떠올렸다.', 0);
    expect(range).toEqual({ rangeStart: 6, rangeEnd: 23 });
  });

  it('정규화 폴백도 searchStart 이전은 매칭하지 않는다', () => {
    const doc = '반복 문장. 반복 문장.';
    const range = createFindRange(doc)('"반복 문장."', '"반복 문장."', 7);
    expect(range).toEqual({ rangeStart: 7, rangeEnd: 13 });
  });
});

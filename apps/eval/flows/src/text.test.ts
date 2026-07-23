import { describe, expect, it } from 'vitest';
import {
  createChunks,
  createFindRange,
  dedupCharacterCandidates,
  extractJsonObjects,
  fuzzyFindMatch,
  renderAdjacentSummary,
  renderMetaBlock,
  renderSummaryForMeta,
} from './text.ts';

describe('createChunks', () => {
  it('1000자 이하는 청크 1개', () => {
    const text = '가'.repeat(999);
    expect(createChunks(text)).toEqual([{ text, start: 0, end: 999 }]);
  });

  it('개행 경계에서 자른다', () => {
    const text = `${'가'.repeat(950)}\n${'나'.repeat(500)}`;
    const chunks = createChunks(text);
    expect(chunks).toHaveLength(2);
    expect(chunks[0].end).toBe(951);
    expect(chunks[1].start).toBe(951);
  });

  it('개행이 없으면 문장 경계에서 자른다', () => {
    const text = `${'가'.repeat(900)}. ${'나'.repeat(500)}`;
    const chunks = createChunks(text);
    expect(chunks[0].end).toBe(902);
  });

  it('청크를 이어 붙이면 원문과 같다', () => {
    const text = Array.from({ length: 50 }, (_, i) => `${i}번째 문장입니다.`)
      .join(' ')
      .repeat(10);
    const chunks = createChunks(text);
    expect(chunks.map((c) => c.text).join('')).toBe(text);
    for (const chunk of chunks) {
      expect(text.slice(chunk.start, chunk.end)).toBe(chunk.text);
    }
  });
});

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
    const doc = '나는 어제 민수 형이 했던 말을 떠올렸다. 다음 문장.';
    const range = createFindRange(doc)('민수 형이했던 말을 떠올렸다.', '민수 형이했던 말을 떠올렸다.', 0);
    expect(range).toEqual({ rangeStart: 6, rangeEnd: 23 });
  });

  it('정규화 폴백도 searchStart 이전은 매칭하지 않는다', () => {
    const doc = '반복 문장. 반복 문장.';
    const range = createFindRange(doc)('"반복 문장."', '"반복 문장."', 7);
    expect(range).toEqual({ rangeStart: 7, rangeEnd: 13 });
  });
});

describe('extractJsonObjects', () => {
  it('연결된 JSON 객체들을 분리한다', () => {
    const buffer = '{"a":1}{"b":"중괄호 } 포함 문자열"}';
    expect([...extractJsonObjects(buffer)]).toEqual(['{"a":1}', '{"b":"중괄호 } 포함 문자열"}']);
  });

  it('이스케이프된 따옴표를 처리한다', () => {
    const buffer = String.raw`{"a":"quote \" brace }"}`;
    expect([...extractJsonObjects(buffer)]).toEqual([String.raw`{"a":"quote \" brace }"}`]);
  });
});

describe('renderSummaryForMeta', () => {
  it('narrative와 메타 라인을 조립한다', () => {
    const rendered = renderSummaryForMeta({
      narrative: '요약 본문',
      characters: ['철수', '영희'],
      pov: '3인칭 제한',
      tense: '과거형',
      location: '서울',
      tone: '긴장감',
    });
    expect(rendered).toBe('요약 본문\n[인물: 철수, 영희] [시점: 3인칭 제한] [시제: 과거형]\n[장소: 서울] [분위기: 긴장감]');
  });
});

describe('dedupCharacterCandidates', () => {
  it('따옴표 제거·대소문자 무시 중복 제거', () => {
    const summaries = [
      { narrative: '', characters: ['"철수"', 'Amy'], pov: '', tense: '', location: '', tone: '' },
      { narrative: '', characters: ['amy', '영희'], pov: '', tense: '', location: '', tone: '' },
    ];
    expect(dedupCharacterCandidates(summaries)).toEqual(['철수', 'Amy', '영희']);
  });
});

describe('renderMetaBlock', () => {
  it('작품 전체 블록을 생성한다', () => {
    const block = renderMetaBlock({
      narrator: { pov: '1인칭 주인공', reliability: '신뢰 가능' },
      setting: '현대 서울',
      themes: ['상실'],
      characters: [{ name: '철수', aliases: ['철'], role: '주인공', arc: '성장' }],
      structure: [{ label: '발단', summary: '시작', tone: '차분' }],
      style: '간결체',
    });
    expect(block).toContain('<작품 전체>');
    expect(block).toContain('- 철수 (철): 주인공. 성장');
    expect(block).toContain('- 발단: 시작 [차분]');
    expect(block).toContain('</작품 전체>');
  });
});

describe('renderMetaBlock 별칭 구조형', () => {
  it('usage가 있으면 병기하고, 구형 문자열 별칭과 공존한다', () => {
    const meta = {
      narrator: { pov: '3인칭 제한', reliability: '신뢰 가능' },
      setting: '현대',
      themes: [],
      characters: [{ name: '하민', aliases: [{ alias: '산호', usage: '마린이 하민을 부르는 애칭' }, '하민씨'], role: '주인공', arc: '' }],
      structure: [],
      style: '',
    };
    const block = renderMetaBlock(meta);
    expect(block).toContain('하민 (산호: 마린이 하민을 부르는 애칭/하민씨)');
  });
});

describe('renderAdjacentSummary', () => {
  const base = { narrative: '두 사람이 카페에서 만난다.', characters: [], pov: '', tense: '', location: '', tone: '' };

  it('구조 필드들이 META 입력과 동일한 형식으로 덧붙는다', () => {
    const rendered = renderAdjacentSummary({ ...base, pov: '3인칭 제한', location: '카페', transitions: '중반부터 회상, 복귀 없음' });
    expect(rendered).toBe('두 사람이 카페에서 만난다.\n[시점: 3인칭 제한]\n[장소: 카페] [장면·시간 구조: 중반부터 회상, 복귀 없음]');
  });

  it('구조 필드가 없으면(구형 저장분) narrative만 반환한다', () => {
    expect(renderAdjacentSummary(base)).toBe('두 사람이 카페에서 만난다.');
    expect(renderAdjacentSummary(undefined)).toBe('');
  });
});

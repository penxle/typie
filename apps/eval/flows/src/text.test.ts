import { describe, expect, it } from 'vitest';
import {
  createChunks,
  createFindRange,
  dedupCharacterCandidates,
  extractJsonObjects,
  fuzzyFindMatch,
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

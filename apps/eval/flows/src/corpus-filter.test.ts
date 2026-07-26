import { describe, expect, it } from 'vitest';
import { corpusSignals, describeSignals, excerptForClassification, isAccepted, rejectedAxes } from './corpus-filter.ts';

// 합성 문장만 쓴다 — 코퍼스 원문은 이용자 텍스트라 레포에 넣지 않는다.
describe('corpusSignals', () => {
  it('화자 이름만 있는 행을 센다', () => {
    const text = ['가온:', '거기 서.', '나린:', '왜.'].join('\n');
    expect(corpusSignals(text).speakerLabelLines).toBe(2);
  });

  it('대괄호 장면 지시를 센다', () => {
    expect(corpusSignals('[위치: 옥상]\n바람이 불었다.').sceneHeadingLines).toBe(1);
  });

  it('번호로 시작하는 절 머리를 센다', () => {
    expect(corpusSignals('01. 첫 조각\n본문이다.\n#02_ 둘째 조각\n본문이다.').numberedSectionLines).toBe(2);
  });

  it('첫머리 안내는 문서 앞쪽에 있을 때만 센다', () => {
    const front = corpusSignals('* 조각글 모음입니다\n본문이다.');
    const buried = corpusSignals([...Array.from({ length: 20 }, () => '본문이다.'), '* 뒤늦은 안내입니다'].join('\n'));
    expect(front.leadingNoteLines).toBe(1);
    expect(buried.leadingNoteLines).toBe(0);
  });

  it('회차 표기를 센다', () => {
    expect(corpusSignals('6장-아직 남은 사람\n본문이다.').episodeMarkers).toBe(1);
    expect(corpusSignals('제3화 재회\n본문이다.').episodeMarkers).toBe(1);
  });

  it('소제목처럼 홀로 놓인 행의 문구를 남긴다', () => {
    const signals = corpusSignals('변주 _ ED1\n그는 걸었다.\n변주 _ ED2\n그는 멈췄다.');
    expect(signals.headings).toEqual(['변주 _ ED1', '변주 _ ED2']);
  });

  it('평서문은 소제목으로 보지 않는다', () => {
    expect(corpusSignals('그는 문을 열었다.\n밖은 어두웠다.').headings).toEqual([]);
  });
});

describe('describeSignals', () => {
  it('걸린 신호만 문장으로 내고 소제목은 목록으로 붙인다', () => {
    const text = ['* 조각글 모음입니다', '01. 첫 조각', '가온:', '안녕.'].join('\n');
    const described = describeSignals(corpusSignals(text));
    expect(described).toContain('화자 이름만 있는 행 1개');
    expect(described).toContain('소제목처럼 홀로 놓인 행');
    expect(described).not.toContain('바깥 링크');
  });
});

describe('excerptForClassification', () => {
  it('짧은 문서는 그대로 넘긴다', () => {
    const text = '짧은 이야기다.';
    expect(excerptForClassification(text)).toBe(text);
  });

  it('긴 문서는 앞·중간·끝 세 곳을 뜬다', () => {
    const text = '가'.repeat(3000) + '나'.repeat(3000) + '다'.repeat(3000) + '라'.repeat(3000);
    const excerpt = excerptForClassification(text);
    expect(excerpt).toContain('=== 문서 앞부분 ===');
    expect(excerpt).toContain('=== 문서 중간 (일부 생략됨) ===');
    expect(excerpt).toContain('=== 문서 끝부분 (일부 생략됨) ===');
    // 끝 발췌가 실제 문서 끝을 담아야 한다 — 여기가 비면 후기·복수 엔딩을 놓친다.
    expect(excerpt.endsWith('라'.repeat(2000))).toBe(true);
  });
});

describe('isAccepted', () => {
  const pass = { narrative: true, singleWork: true, selfContained: true, original: true };

  it('네 축을 모두 통과해야 받는다', () => {
    expect(isAccepted(pass)).toBe(true);
    expect(isAccepted({ ...pass, singleWork: false })).toBe(false);
  });

  it('걸린 축을 이름으로 돌려준다', () => {
    expect(rejectedAxes({ ...pass, narrative: false, selfContained: false })).toEqual(['narrative', 'selfContained']);
    expect(rejectedAxes(pass)).toEqual([]);
  });
});

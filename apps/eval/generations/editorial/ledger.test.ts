import { describe, expect, it } from 'vitest';
import { grepBefore, mergeRanges, readRanges, turnNote, uncovered, withinRead } from './ledger.ts';
import type { ToolRecord } from './ledger.ts';

const FILE = 'manuscript/doc-1.txt';
const OTHER = 'manuscript/doc-2.txt';

const READS: ToolRecord[] = [
  { turn: 0, tool: 'read', file: FILE, start: 0, end: 100 },
  { turn: 1, tool: 'read', file: FILE, start: 80, end: 200 },
  { turn: 3, tool: 'grep', file: FILE, pattern: '광역', total: 2 },
];

// 파일시스템 전환 전 원장 — file 필드가 없다. 유일 원고의 기록으로 해석해야 한다.
const LEGACY: ToolRecord[] = [
  { turn: 0, tool: 'read', start: 0, end: 100 },
  { turn: 2, tool: 'grep', pattern: 'x', total: 1 },
];

describe('mergeRanges', () => {
  it('겹치는 범위를 병합·정렬한다', () => {
    expect(
      mergeRanges([
        { start: 80, end: 200 },
        { start: 0, end: 100 },
      ]),
    ).toEqual([{ start: 0, end: 200 }]);
  });
});

describe('readRanges', () => {
  it('해당 파일의 기록만 센다 — 다른 파일 기록이 섞여도 각자 계산된다', () => {
    const mixed: ToolRecord[] = [...READS, { turn: 5, tool: 'read', file: OTHER, start: 500, end: 600 }];
    expect(readRanges(mixed, FILE)).toEqual([{ start: 0, end: 200 }]);
    expect(readRanges(mixed, OTHER)).toEqual([{ start: 500, end: 600 }]);
  });

  it('구원장의 무필드 기록은 어느 파일로도 매치된다', () => {
    expect(readRanges(LEGACY, FILE)).toEqual([{ start: 0, end: 100 }]);
  });
});

describe('uncovered', () => {
  it('미열람 구간을 돌려준다', () => {
    expect(uncovered(300, readRanges(READS, FILE), [])).toEqual([{ start: 200, end: 300 }]);
  });

  // 후기 등 제외 구간은 커버리지 의무에서 뺀다.
  it('제외 구간은 미열람으로 치지 않는다', () => {
    expect(uncovered(300, readRanges(READS, FILE), [{ start: 200, end: 300 }])).toEqual([]);
  });
});

describe('withinRead', () => {
  it('해당 파일의 열람 범위 안 인용만 허용한다', () => {
    expect(withinRead(READS, 150, 180, FILE)).toBe(true);
    expect(withinRead(READS, 190, 250, FILE)).toBe(false);
    expect(withinRead(READS, 150, 180, OTHER)).toBe(false);
  });
});

describe('grepBefore', () => {
  it('해당 턴 이전, 해당 파일의 grep 존재를 판정한다', () => {
    expect(grepBefore(READS, 4, FILE)).toBe(true);
    expect(grepBefore(READS, 3, FILE)).toBe(false);
    expect(grepBefore(READS, 4, OTHER)).toBe(false);
    expect(grepBefore(LEGACY, 3, FILE)).toBe(true);
  });
});

describe('turnNote', () => {
  it('텍스트·thinking 블록과 행동 줄을 남긴다', () => {
    const note = turnNote(
      'plan-draft',
      3,
      [
        { type: 'thinking', thinking: '구조를 먼저 본다', signature: 'SIG' },
        { type: 'text', text: '축을 세우겠습니다' },
        { type: 'tool_use', id: 'a', name: 'read', input: { start: 0, end: 10 } },
      ],
      ['write output/plan.yaml (1,024자)', 'submit_plan → 반려 2건'],
    );
    expect(note.stage).toBe('plan-draft');
    expect(note.turn).toBe(3);
    expect(note.thinking).toBe('구조를 먼저 본다');
    expect(note.text).toBe('축을 세우겠습니다');
    expect(note.submissions).toEqual(['write output/plan.yaml (1,024자)', 'submit_plan → 반려 2건']);
  });

  it('긴 텍스트는 상한에서 자르고, 빈 thinking(omitted 기본값)은 빈 문자열로 남는다', () => {
    const note = turnNote(
      'execute',
      0,
      [
        { type: 'thinking', thinking: '', signature: 'SIG' },
        { type: 'text', text: '가'.repeat(3000) },
      ],
      [],
    );
    expect(note.thinking).toBe('');
    expect(note.text.length).toBe(2001);
    expect(note.text.endsWith('…')).toBe(true);
  });
});

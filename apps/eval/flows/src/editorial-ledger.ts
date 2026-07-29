// 단계 원장 — 스펙 §4. 도구 호출 전건과 사건을 남기고, 열람 범위·커버리지의 진실 원천이 된다.
// 워크플로 리플레이 시 캐시된 도구 실행 결과에서 재구성되므로 순수해야 한다.
import type { Range } from './editorial-types.ts';

export type ToolRecord =
  | { turn: number; tool: 'read'; start: number; end: number }
  | { turn: number; tool: 'grep'; pattern: string; total: number }
  | { turn: number; tool: 'search'; query: string; hits: number };

export type LedgerEvent = { turn: number; kind: string; detail: string };

// leaked는 직렬화 오염으로 중앙 차단된 제출의 원문 — 오염 입력은 대화에서 제거되므로
// 여기가 유일한 사후 진단 경로이고, 유실 축 판정(cleared 제외)의 근거다.
export type StageLedger = { tools: ToolRecord[]; events: LedgerEvent[]; leaked: { turn: number; name: string; input: unknown }[] };

export const emptyLedger = (): StageLedger => ({ tools: [], events: [], leaked: [] });

export const mergeRanges = (ranges: Range[]): Range[] => {
  const sorted = ranges.toSorted((a, b) => a.start - b.start);
  const out: Range[] = [];
  for (const r of sorted) {
    const last = out.at(-1);
    if (last && r.start <= last.end) last.end = Math.max(last.end, r.end);
    else out.push({ ...r });
  }
  return out;
};

export const readRanges = (tools: ToolRecord[]): Range[] =>
  mergeRanges(
    tools.filter((t): t is Extract<ToolRecord, { tool: 'read' }> => t.tool === 'read').map((t) => ({ start: t.start, end: t.end })),
  );

export const uncovered = (length: number, covered: Range[], excluded: Range[]): Range[] => {
  const filled = mergeRanges([...covered, ...excluded]);
  const gaps: Range[] = [];
  let cursor = 0;
  for (const r of filled) {
    if (r.start > cursor) gaps.push({ start: cursor, end: r.start });
    cursor = Math.max(cursor, r.end);
  }
  if (cursor < length) gaps.push({ start: cursor, end: length });
  return gaps;
};

export const withinRead = (tools: ToolRecord[], start: number, end: number): boolean =>
  readRanges(tools).some((r) => start >= r.start && end <= r.end);

export const grepBefore = (tools: ToolRecord[], turn: number): boolean => tools.some((t) => t.tool === 'grep' && t.turn < turn);

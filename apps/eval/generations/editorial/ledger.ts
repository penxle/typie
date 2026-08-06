// 단계 원장. 도구 호출 전건과 사건을 남기고, 열람 범위·커버리지의 진실 원천이 된다.
// 워크플로 리플레이 시 캐시된 도구 실행 결과에서 재구성되므로 순수해야 한다.
import type { ToolRecord } from '../../core/contracts.ts';
import type { Range } from './types.ts';

export type { ToolRecord } from '../../core/contracts.ts';

export type LedgerEvent = { turn: number; kind: string; detail: string };

// 턴 단위 진행 기록 — 어드민 실행 상세의 실시간 열람이 읽는다. call_cache는 블랙박스
// 캐시라 화면이 읽지 않는다는 원칙 위에서, 화면에 보일 것은 원장이 남긴다.
// thinking은 display: summarized로 받은 요약 사고다 — 5계열 기본은 omitted(빈 본문)라
// 요청 옵트인이 있어야 채워진다.
export type TurnNote = { stage: string; turn: number; thinking: string; text: string; submissions: string[] };

// 초안 흐름 전환으로 제출은 더 이상 내용을 나르지 않아 새 오염 원문이 생기지 않는다 —
// leaked는 구계약 실행의 원장을 읽기 위한 유산 필드로만 남는다(새 코드는 쓰지 않는다).
export type StageLedger = {
  tools: ToolRecord[];
  events: LedgerEvent[];
  leaked: { turn: number; name: string; input: unknown }[];
  turns: TurnNote[];
  // 스테이지 완료 시점의 scratch/ 스냅샷(파일당 4,000자 캡). 실행 중 실시간성은 턴 기록이
  // 담당한다 — 스냅샷은 모델이 무엇을 메모하며 일했는지의 사후 열람용이다.
  scratchFiles?: { path: string; content: string }[];
};

export const emptyLedger = (): StageLedger => ({ tools: [], events: [], leaked: [], turns: [] });

// 턴 기록 상한. 원장 행은 턴마다 통째로 다시 쓰므로 상한이 없으면 누적 쓰기가 턴 수의
// 제곱으로 불고, 상세 화면은 진행 중 3초 폴링마다 원장 전체를 다시 받는다. 여기는 진행
// 스트림이지 아카이브가 아니다 — 전문은 산출물·단계 아티팩트가 이미 보존한다.
// 상한값 자체는 재량이다: 비용은 값에 선형이니 잘림이 거슬리면 올려도 된다.
const TURN_TEXT_CAP = 2000;

// 캐시된 턴 출력만으로 만든다 — 리플레이 재구성의 순수성이 지켜져야 한다. actions는
// 루프가 요약한 이 턴의 행동 줄(초안 연산·제출 결과)이다. 필드명 submissions는 배포된
// 화면과의 호환을 위해 유지한다.
export const turnNote = (stage: string, turn: number, content: unknown[], actions: string[]): TurnNote => {
  const text = content
    .filter((b): b is { type: 'text'; text: string } => {
      const block = b as { type?: unknown; text?: unknown };
      return block.type === 'text' && typeof block.text === 'string';
    })
    .map((b) => b.text)
    .join('\n');
  const thinking = content
    .filter((b): b is { type: 'thinking'; thinking: string } => {
      const block = b as { type?: unknown; thinking?: unknown };
      return block.type === 'thinking' && typeof block.thinking === 'string';
    })
    .map((b) => b.thinking)
    .join('\n');
  const cap = (s: string) => (s.length > TURN_TEXT_CAP ? `${s.slice(0, TURN_TEXT_CAP)}…` : s);
  return { stage, turn, thinking: cap(thinking), text: cap(text), submissions: actions };
};

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

// 구원장(파일시스템 전환 전)의 기록에는 file이 없다 — 유일 원고에 대한 기록으로 해석한다.
const forFile = (r: { file?: string }, file: string): boolean => r.file === undefined || r.file === file;

export const readRanges = (tools: ToolRecord[], file: string): Range[] =>
  mergeRanges(
    tools
      .filter((t): t is Extract<ToolRecord, { tool: 'read' }> => t.tool === 'read')
      .filter((t) => forFile(t, file))
      .map((t) => ({ start: t.start, end: t.end })),
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

export const withinRead = (tools: ToolRecord[], start: number, end: number, file: string): boolean =>
  readRanges(tools, file).some((r) => start >= r.start && end <= r.end);

export const grepBefore = (tools: ToolRecord[], turn: number, file: string): boolean =>
  tools.some((t) => t.tool === 'grep' && forFile(t, file) && t.turn < turn);

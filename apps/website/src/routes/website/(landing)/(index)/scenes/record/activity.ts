// 잔디가 하루씩 채워지는 구간 — 도중에 멈춰도 그 자체로 온전하다
export const FILL_RANGE = [0, 0.74] as const;

export type Cell = { level: number; day: number };
export type MonthSpan = { month: number; start: number; end: number; at: number };

// 지난 1년간의 기록 — 스크럽은 결정적이어야 하므로 종료일을 고정한다
const END = Date.UTC(2026, 8, 7);
const DAY = 86_400_000;
export const DAYS = 365;

const START = END - (DAYS - 1) * DAY;
const FIRST_DAY = new Date(START).getUTCDay();
const STREAK_TAIL = 118;

const columnOf = (index: number) => Math.floor((index + FIRST_DAY) / 7);

const build = (): readonly Cell[] => {
  let seed = 20_260_907;
  const next = () => {
    seed = (seed * 1_664_525 + 1_013_904_223) % 4_294_967_296;
    return seed / 4_294_967_296;
  };

  return Array.from({ length: DAYS }, (_, index) => {
    const date = new Date(START + index * DAY);
    const growth = index / DAYS;
    const level =
      index < DAYS - STREAK_TAIL && next() < 0.32 - growth * 0.16 ? 0 : Math.min(5, 1 + Math.floor(next() * (0.4 + growth * 0.6) * 5));
    return { level, day: date.getUTCDay() };
  });
};

export const CELLS = build();

export type MonthCell = { level: number; at: number };
export type MonthBlock = { label: string; lead: readonly number[]; cells: readonly MonthCell[]; at: number };

// apps/api/src/utils/image-generation.tsx — 달마다 1일부터 말일까지를 7열로 감아 그린다
export const MONTH_BLOCKS: readonly MonthBlock[] = (() => {
  const blocks = new Map<string, MonthBlock>();

  for (const [index, cell] of CELLS.entries()) {
    const date = new Date(START + index * DAY);
    const key = `${date.getUTCFullYear()}-${date.getUTCMonth()}`;
    let block = blocks.get(key);

    if (!block) {
      const first = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), 1));
      const lead = Array.from({ length: first.getUTCDay() }, (_, offset) => offset);
      block = { label: `${date.getUTCMonth() + 1}월`, lead, cells: [], at: index };
      blocks.set(key, block);
    }

    (block.cells as MonthCell[]).push({ level: cell.level, at: index });
  }

  return [...blocks.values()];
})();

export const MONTH_SPANS: readonly MonthSpan[] = (() => {
  const spans: MonthSpan[] = [];
  let previous = -1;

  for (let index = 0; index < DAYS; index += 1) {
    const month = new Date(START + index * DAY).getUTCMonth() + 1;
    if (month === previous) continue;

    const column = columnOf(index);
    const last = spans.at(-1);
    if (last) last.end = column - 1;
    spans.push({ month, start: column, end: column, at: index });
    previous = month;
  }

  const last = spans.at(-1);
  if (last) last.end = columnOf(DAYS - 1);

  return spans;
})();

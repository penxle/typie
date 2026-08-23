export type MarginMode = 'column' | 'popover';

export const GUTTER = 100;
export const COLUMN_WIDTH = 368;
// 레일과 본문 사이(RAIL_TEXT_GAP 44)는 이 간격보다 넉넉해야 한다 — 왼쪽은 빈 여백이 아니라
// 연속된 유색 막대가 붙어 있어 같은 거리도 더 좁게 읽힌다.
export const COLUMN_GAP = 28;

const HYSTERESIS = 40;

// 임계 근처에서 모드가 튀지 않도록 진입과 이탈의 문턱을 벌린다
export const resolveMode = (available: number, bodyWidth: number, current: MarginMode): MarginMode => {
  const need = GUTTER + bodyWidth + COLUMN_GAP + COLUMN_WIDTH;
  if (current === 'column') return available >= need - HYSTERESIS ? 'column' : 'popover';
  return available >= need + HYSTERESIS ? 'column' : 'popover';
};

export type RoundOption = { id: string; ordinal: number; tierLabel: string; issueCount: number; createdAt: string };

export const roundLabel = (option: RoundOption): string => `${option.ordinal}회차 · ${option.tierLabel} · 피드백 ${option.issueCount}`;

type IssueBrief = { index: number; trait: string };
type Pattern = { theme: string | null; body: string; issues: readonly IssueBrief[] };
type Priority = { body: string; issues: readonly IssueBrief[] };

export type ThreadCallouts = {
  pattern: { theme: string; count: number } | null;
  priority: { rank: number; total: number; body: string } | null;
};

export const describeThread = (issueIndex: number, patterns: readonly Pattern[], priorities: readonly Priority[]): ThreadCallouts => {
  const owner = patterns.find((pattern) => pattern.issues.some((issue) => issue.index === issueIndex));
  const at = priorities.findIndex((priority) => priority.issues.some((issue) => issue.index === issueIndex));

  return {
    pattern: owner?.theme ? { theme: owner.theme, count: owner.issues.length } : null,
    priority: at === -1 ? null : { rank: at + 1, total: priorities.length, body: priorities[at].body },
  };
};

export const edgeJumpLabel = (edge: 'top' | 'bottom', count: number): string =>
  edge === 'top' ? `위로 피드백 ${count}개` : `아래로 피드백 ${count}개`;

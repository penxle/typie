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

// 카드 헤더 우측이 무엇을 세우는가. 접힘은 정적 요약이고 펼침은 액션이라는 규칙이 다섯 상태에 걸쳐 있어
// 마크업 안 삼항으로 두면 읽히지 않는다.
export type CardThreadState = 'OPEN' | 'CLOSED' | 'RESOLVED' | 'WITHDRAWN';

export type CardHeaderSlot = { comments: boolean; state: boolean; action: 'close' | 'reopen' | null };

export const cardHeaderSlot = (state: CardThreadState | null, expanded: boolean, commentCount: number): CardHeaderSlot => {
  // 강점은 스레드가 없다 — 셀 댓글도 되돌릴 상태도 없다
  if (state === null) return { comments: false, state: false, action: null };
  if (!expanded) return { comments: commentCount > 0, state: state !== 'OPEN', action: null };
  // 되돌릴 수 있는 두 상태에서는 액션만 세운다 — 액션의 존재가 곧 상태 표지라 라벨을 겹칠 이유가 없다
  if (state === 'OPEN') return { comments: false, state: false, action: 'close' };
  if (state === 'CLOSED') return { comments: false, state: false, action: 'reopen' };
  return { comments: false, state: true, action: null };
};

// sessionId는 리뷰를 진행한 대화 — 대화가 지워진 라운드는 null이다
export type RoundOption = {
  id: string;
  ordinal: number;
  tierLabel: string;
  issueCount: number;
  sessionId: string | null;
  createdAt: string;
};

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

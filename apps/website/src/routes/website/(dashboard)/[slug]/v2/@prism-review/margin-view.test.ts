import { describe, expect, it } from 'vitest';
import { cardHeaderSlot, COLUMN_GAP, COLUMN_WIDTH, describeThread, groupRoundsByLineage, GUTTER, resolveMode } from './margin-view.ts';

describe('레이아웃 상수', () => {
  it('치수가 조용히 바뀌면 레일 자리와 컬럼 임계가 어긋난다', () => {
    expect(GUTTER).toBe(100);
    expect(COLUMN_WIDTH).toBe(368);
    expect(COLUMN_GAP).toBe(28);
  });
});

describe('resolveMode', () => {
  const body = 640;
  const need = GUTTER + body + COLUMN_GAP + COLUMN_WIDTH;

  it('자리가 넉넉하면 컬럼이다', () => {
    expect(resolveMode(need + 100, body, 'popover')).toBe('column');
  });

  it('자리가 모자라면 팝오버다', () => {
    expect(resolveMode(need - 100, body, 'column')).toBe('popover');
  });

  it('실제로 들어가지 않는 폭에서는 컬럼을 유지하지 않는다', () => {
    expect(resolveMode(need - 0.1, body, 'column')).toBe('popover');
    expect(resolveMode(need, body, 'column')).toBe('column');
  });

  it('진입 여유 안에서는 현재 모드를 유지한다', () => {
    expect(resolveMode(need, body, 'column')).toBe('column');
    expect(resolveMode(need + 7, body, 'popover')).toBe('popover');
  });

  it('8px 여유가 생기면 컬럼에 진입한다', () => {
    expect(resolveMode(need + 8, body, 'popover')).toBe('column');
  });
});

describe('describeThread', () => {
  const patterns = [
    {
      theme: '부사 의존',
      body: '',
      issues: [
        { index: 0, trait: '동어 반복' },
        { index: 2, trait: '군더더기' },
      ],
    },
  ];
  const priorities = [
    { body: '먼저 손볼 것', issues: [{ index: 5, trait: '시점 흔들림' }] },
    { body: '그다음', issues: [{ index: 0, trait: '동어 반복' }] },
  ];

  it('소속 패턴과 순서를 찾아 준다', () => {
    const view = describeThread(0, patterns, priorities);
    expect(view.pattern).toEqual({ theme: '부사 의존', count: 2 });
    expect(view.priority).toEqual({ rank: 2, total: 2, body: '그다음' });
  });

  it('소속이 없으면 둘 다 없다', () => {
    expect(describeThread(9, patterns, priorities)).toEqual({ pattern: null, priority: null });
  });

  it('theme이 빈 패턴은 콜아웃을 만들지 않는다', () => {
    const themeless = [{ theme: null, body: '', issues: [{ index: 0, trait: '동어 반복' }] }];
    expect(describeThread(0, themeless, []).pattern).toBeNull();
  });
});

describe('cardHeaderSlot', () => {
  const STATES = ['OPEN', 'CLOSED', 'RESOLVED', 'WITHDRAWN'] as const;

  it('접힌 카드는 어떤 상태에서도 액션을 세우지 않는다', () => {
    for (const state of STATES) {
      expect(cardHeaderSlot(state, false, 2).action).toBeNull();
    }
  });

  it('접힘에서 댓글은 있을 때만, 상태는 종결일 때만 선다', () => {
    expect(cardHeaderSlot('OPEN', false, 0)).toEqual({ comments: false, state: false, action: null });
    expect(cardHeaderSlot('OPEN', false, 2)).toEqual({ comments: true, state: false, action: null });
    expect(cardHeaderSlot('CLOSED', false, 0)).toEqual({ comments: false, state: true, action: null });
    expect(cardHeaderSlot('CLOSED', false, 2)).toEqual({ comments: true, state: true, action: null });
    expect(cardHeaderSlot('RESOLVED', false, 3)).toEqual({ comments: true, state: true, action: null });
    expect(cardHeaderSlot('WITHDRAWN', false, 0)).toEqual({ comments: false, state: true, action: null });
  });

  it('펼침에서는 댓글 수를 세우지 않는다', () => {
    for (const state of STATES) {
      expect(cardHeaderSlot(state, true, 5).comments).toBe(false);
    }
  });

  it('되돌릴 수 있는 상태에만 액션이 서고, 그때는 라벨을 세우지 않는다', () => {
    expect(cardHeaderSlot('OPEN', true, 0)).toEqual({ comments: false, state: false, action: 'close' });
    expect(cardHeaderSlot('CLOSED', true, 0)).toEqual({ comments: false, state: false, action: 'reopen' });
  });

  it('재리뷰가 처분한 상태는 액션 없이 라벨만 선다', () => {
    expect(cardHeaderSlot('RESOLVED', true, 0)).toEqual({ comments: false, state: true, action: null });
    expect(cardHeaderSlot('WITHDRAWN', true, 0)).toEqual({ comments: false, state: true, action: null });
  });

  it('스레드가 없는 강점은 아무것도 세우지 않는다', () => {
    expect(cardHeaderSlot(null, false, 0)).toEqual({ comments: false, state: false, action: null });
    expect(cardHeaderSlot(null, true, 0)).toEqual({ comments: false, state: false, action: null });
  });
});

describe('groupRoundsByLineage', () => {
  it('계보별로 묶고 그룹 순서는 최신 회차 우선, 그룹 안은 서수 내림차', () => {
    const rounds = [
      { id: 'r3', ordinal: 1, tierLabel: '빠른 검토', issueCount: 1, sessionId: null, createdAt: '2026-08-03T00:00:00Z', lineageId: 'B' },
      { id: 'r2', ordinal: 2, tierLabel: '심층 검토', issueCount: 1, sessionId: null, createdAt: '2026-08-02T00:00:00Z', lineageId: 'A' },
      { id: 'r1', ordinal: 1, tierLabel: '심층 검토', issueCount: 1, sessionId: null, createdAt: '2026-08-01T00:00:00Z', lineageId: 'A' },
    ];
    expect(groupRoundsByLineage(rounds).map((g) => [g.lineageId, g.rounds.map((r) => r.id)])).toEqual([
      ['B', ['r3']],
      ['A', ['r2', 'r1']],
    ]);
    expect(groupRoundsByLineage(rounds)[1]).toMatchObject({ tierLabel: '심층 검토', startedAt: '2026-08-01T00:00:00Z' });
  });
});

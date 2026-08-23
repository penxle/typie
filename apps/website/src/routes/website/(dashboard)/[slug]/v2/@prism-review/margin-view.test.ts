import { describe, expect, it } from 'vitest';
import { COLUMN_GAP, COLUMN_WIDTH, describeThread, GUTTER, resolveMode, roundLabel } from './margin-view.ts';

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

  it('임계 근처에서는 현재 모드를 유지한다', () => {
    expect(resolveMode(need - 20, body, 'column')).toBe('column');
    expect(resolveMode(need + 20, body, 'popover')).toBe('popover');
  });

  it('이탈 문턱은 정확히 40이다', () => {
    expect(resolveMode(need - 40, body, 'column')).toBe('column');
    expect(resolveMode(need - 41, body, 'column')).toBe('popover');
  });

  it('진입 문턱은 정확히 40이다', () => {
    expect(resolveMode(need + 40, body, 'popover')).toBe('column');
    expect(resolveMode(need + 39, body, 'popover')).toBe('popover');
  });
});

describe('roundLabel', () => {
  it('회차·깊이·피드백 수를 잇는다', () => {
    expect(roundLabel({ id: 'r1', ordinal: 3, tierLabel: '심층 검토', issueCount: 9, createdAt: '2026-08-20' })).toBe(
      '3회차 · 심층 검토 · 피드백 9',
    );
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

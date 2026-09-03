import { describe, expect, it } from 'vitest';
import {
  contentMotionOffset,
  lanePresentation,
  marginInsets,
  marginMotionDuration,
  marginMotionTarget,
  nextMarginReserved,
  resolveRoundSwap,
  targetColumnLeft,
  visibleAreaCenterY,
} from './margin-motion.ts';
import { COLUMN_GAP, COLUMN_WIDTH, GUTTER } from './margin-view.ts';

describe('marginInsets', () => {
  it('하나의 진행률로 양쪽 예약 폭을 보간한다', () => {
    expect(marginInsets(0)).toEqual({ left: 0, right: 0 });
    expect(marginInsets(0.5)).toEqual({ left: GUTTER / 2, right: (COLUMN_WIDTH + COLUMN_GAP) / 2 });
    expect(marginInsets(1)).toEqual({ left: GUTTER, right: COLUMN_WIDTH + COLUMN_GAP });
  });

  it('범위를 벗어난 진행률을 끝점으로 제한한다', () => {
    expect(marginInsets(-0.2)).toEqual(marginInsets(0));
    expect(marginInsets(1.2)).toEqual(marginInsets(1));
    expect(lanePresentation(-0.2)).toEqual(lanePresentation(0));
    expect(lanePresentation(1.2)).toEqual(lanePresentation(1));
  });
});

describe('nextMarginReserved', () => {
  it('준비되지 않은 회차 교체 중에는 폭을 유지하고 선택을 닫을 때만 놓는다', () => {
    expect(nextMarginReserved(false, 'round-a', false)).toBe(false);
    expect(nextMarginReserved(false, 'round-a', true)).toBe(true);
    expect(nextMarginReserved(true, 'round-b', false)).toBe(true);
    expect(nextMarginReserved(true, null, true)).toBe(false);
  });
});

describe('marginMotionTarget', () => {
  it('배치가 준비된 컬럼에 예약 폭이 있을 때만 열린다', () => {
    expect(marginMotionTarget('column', false, true, true)).toBe(1);
    expect(marginMotionTarget('column', false, true, false)).toBe(0);
    expect(marginMotionTarget('column', false, false, true)).toBe(0);
    expect(marginMotionTarget('popover', false, true, true)).toBe(0);
    expect(marginMotionTarget('column', true, true, true)).toBe(0);
  });
});

describe('marginMotionDuration', () => {
  it('모션 감소 환경에서는 즉시 끝점에 도달한다', () => {
    expect(marginMotionDuration(false)).toBeGreaterThan(0);
    expect(marginMotionDuration(true)).toBe(0);
  });
});

describe('targetColumnLeft', () => {
  it('inset 전환 내내 같은 최종 좌표를 돌려준다', () => {
    const closedPageRight = 900;
    const shift = (GUTTER - (COLUMN_WIDTH + COLUMN_GAP)) / 2;
    const expected = closedPageRight + shift + COLUMN_GAP;

    expect(targetColumnLeft(closedPageRight, 0)).toBe(expected);
    expect(targetColumnLeft(closedPageRight + shift / 2, 0.5)).toBe(expected);
    expect(targetColumnLeft(closedPageRight + shift, 1)).toBe(expected);
  });
});

describe('contentMotionOffset', () => {
  it('전환 중 목표가 바뀌어도 현재 화면 위치를 이어 간다', () => {
    const shift = 148;
    const progress = 0.4;
    const openingLayoutLeft = -shift;
    const closingLayoutLeft = 0;

    const openingVisualLeft = openingLayoutLeft + contentMotionOffset(1, progress, shift);
    const closingVisualLeft = closingLayoutLeft + contentMotionOffset(0, progress, shift);

    expect(openingVisualLeft).toBeCloseTo(closingVisualLeft);
  });
});

type RoundSwapInput = {
  selectedRoundId: string | null;
  presentedRoundId: string | null;
  loadedRoundId: string | null;
  visibilityProgress: number;
  prepared: boolean;
  failed: boolean;
};

const swappingFromAToB = (overrides: Partial<RoundSwapInput> = {}): RoundSwapInput => ({
  selectedRoundId: 'round-b',
  presentedRoundId: 'round-a',
  loadedRoundId: null,
  visibilityProgress: 1,
  prepared: false,
  failed: false,
  ...overrides,
});

describe('회차 교체 상태', () => {
  it('완료된 상태를 다시 계산할 때 같은 객체를 유지해 reactive effect를 재실행하지 않는다', () => {
    const idle = { phase: 'idle' } as const;
    const resolution = resolveRoundSwap(idle, swappingFromAToB({ selectedRoundId: 'round-a', loadedRoundId: 'round-a' }));

    expect(resolution.state).toBe(idle);
  });

  it('fade-out 안에 데이터가 오면 스피너 없이 배치를 마친 뒤 fade-in한다', () => {
    let resolution = resolveRoundSwap({ phase: 'idle' }, swappingFromAToB());
    expect(resolution).toMatchObject({ state: { phase: 'fading-out', targetId: 'round-b' }, visibilityTarget: 0 });

    resolution = resolveRoundSwap(resolution.state, swappingFromAToB({ loadedRoundId: 'round-b', visibilityProgress: 0 }));
    expect(resolution).toEqual({
      state: { phase: 'preparing', targetId: 'round-b', spinnerVisible: false },
      replacePresented: true,
      restoreSelection: false,
      visibilityTarget: 0,
    });

    resolution = resolveRoundSwap(resolution.state, swappingFromAToB({ presentedRoundId: 'round-b', loadedRoundId: 'round-b' }));
    expect(resolution.state).toMatchObject({ phase: 'preparing', spinnerVisible: false });

    resolution = resolveRoundSwap(
      resolution.state,
      swappingFromAToB({ presentedRoundId: 'round-b', loadedRoundId: 'round-b', visibilityProgress: 0, prepared: true }),
    );
    expect(resolution).toMatchObject({ state: { phase: 'fading-in', targetId: 'round-b' }, visibilityTarget: 1 });

    resolution = resolveRoundSwap(
      resolution.state,
      swappingFromAToB({ presentedRoundId: 'round-b', loadedRoundId: 'round-b', visibilityProgress: 1, prepared: true }),
    );
    expect(resolution.state).toEqual({ phase: 'idle' });
  });

  it('fade-out이 끝나도 데이터가 없으면 배치가 끝날 때까지 스피너를 유지한다', () => {
    let resolution = resolveRoundSwap({ phase: 'fading-out', targetId: 'round-b' }, swappingFromAToB({ visibilityProgress: 0 }));
    expect(resolution.state).toEqual({ phase: 'waiting', targetId: 'round-b' });

    resolution = resolveRoundSwap(resolution.state, swappingFromAToB({ loadedRoundId: 'round-b', visibilityProgress: 0 }));
    expect(resolution.state).toEqual({ phase: 'preparing', targetId: 'round-b', spinnerVisible: true });

    resolution = resolveRoundSwap(
      resolution.state,
      swappingFromAToB({ presentedRoundId: 'round-b', loadedRoundId: 'round-b', visibilityProgress: 0, prepared: true }),
    );
    expect(resolution).toMatchObject({ state: { phase: 'fading-in' }, visibilityTarget: 1 });
  });

  it('대상 회차 로드가 실패하면 기존 회차를 복구하고 스피너를 내린다', () => {
    const resolution = resolveRoundSwap(
      { phase: 'waiting', targetId: 'round-b' },
      swappingFromAToB({ visibilityProgress: 0, failed: true }),
    );

    expect(resolution).toEqual({
      state: { phase: 'fading-in', targetId: 'round-a' },
      replacePresented: false,
      restoreSelection: true,
      visibilityTarget: 1,
    });
  });

  it('실패 복구가 반영되기 전 다시 계산되어도 같은 fading-in 상태를 유지한다', () => {
    const restoring = { phase: 'fading-in', targetId: 'round-a' } as const;
    const resolution = resolveRoundSwap(restoring, swappingFromAToB({ visibilityProgress: 0, failed: true }));

    expect(resolution.state).toBe(restoring);
    expect(resolution.restoreSelection).toBe(false);
  });

  it('fade-out 중 원래 회차를 다시 고르면 현재 진행률에서 되돌린다', () => {
    const resolution = resolveRoundSwap(
      { phase: 'fading-out', targetId: 'round-b' },
      swappingFromAToB({ selectedRoundId: 'round-a', visibilityProgress: 0.4 }),
    );

    expect(resolution).toMatchObject({ state: { phase: 'fading-in', targetId: 'round-a' }, visibilityTarget: 1 });
  });

  it('스피너를 에디터 visible area의 세로 중앙에 놓는다', () => {
    expect(visibleAreaCenterY(100, 600, { topInset: 80, bottomInset: 40 })).toBe(420);
    expect(visibleAreaCenterY(100, 600, { topInset: 0, bottomInset: 0 })).toBe(400);
  });
});

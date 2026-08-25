import { describe, expect, it } from 'vitest';
import {
  contentMotionOffset,
  lanePresentation,
  marginInsets,
  marginMotionDuration,
  marginMotionTarget,
  nextMarginReserved,
  resolvePresentedRoundId,
  targetColumnLeft,
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

describe('resolvePresentedRoundId', () => {
  it('닫힘이 끝나기 전까지만 마지막 회차를 유지한다', () => {
    expect(resolvePresentedRoundId(null, 'round-a', 1)).toBe('round-a');
    expect(resolvePresentedRoundId(null, 'round-a', 0.2)).toBe('round-a');
    expect(resolvePresentedRoundId(null, 'round-a', 0)).toBeNull();
    expect(resolvePresentedRoundId('round-b', 'round-a', 0.2)).toBe('round-b');
  });

  it('닫히는 중 다시 열면 표시 회차를 비우지 않고 목표를 되돌린다', () => {
    const closingProgress = 0.4;
    expect(resolvePresentedRoundId(null, 'round-a', closingProgress)).toBe('round-a');
    expect(marginMotionTarget('column', false, false, true)).toBe(0);

    expect(resolvePresentedRoundId('round-a', 'round-a', closingProgress)).toBe('round-a');
    expect(marginMotionTarget('column', false, true, true)).toBe(1);
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

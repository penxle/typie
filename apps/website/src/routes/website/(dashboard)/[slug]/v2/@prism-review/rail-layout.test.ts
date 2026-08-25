import { describe, expect, it } from 'vitest';
import { layoutRailHitTargets, layoutRails, maxRailLanes, RAIL_CHIP_SIZE, RAIL_TEXT_GAP, RAIL_WIDTH } from './rail-layout.ts';
import type { PlacedRail, RailSpan } from './rail-layout.ts';

const span = (id: string, top: number, height: number): RailSpan => ({
  id,
  itemId: id.split(':')[0],
  number: 1,
  tone: 'open',
  top,
  height,
});

type Box = { left: number; top: number; right: number; bottom: number };

const intersects = (a: Box, b: Box) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

const barBox = (rail: PlacedRail): Box => ({
  left: rail.left,
  top: rail.top,
  right: rail.left + RAIL_WIDTH,
  bottom: rail.top + rail.height,
});

const chipBox = (rail: PlacedRail): Box => ({
  left: rail.chipLeft,
  top: rail.chipTop,
  right: rail.chipLeft + RAIL_CHIP_SIZE,
  bottom: rail.chipTop + RAIL_CHIP_SIZE,
});

describe('layoutRails', () => {
  it('겹치지 않는 막대는 모두 0번 레인에 선다', () => {
    const placed = layoutRails([span('a:0', 0, 20), span('b:0', 40, 20)], 100);
    expect(placed.map((rail) => rail.lane)).toEqual([0, 0]);
  });

  it('겹치는 막대는 바깥 레인으로 밀린다', () => {
    const placed = layoutRails([span('a:0', 0, 50), span('b:0', 10, 50)], 100);
    expect(placed.map((rail) => rail.lane)).toEqual([0, 1]);
  });

  it('레인이 소진되면 마지막 레인에 겹쳐 싣는다', () => {
    const spans = Array.from({ length: 12 }, (_, index) => span(`s${index}:0`, index * 2, 200));
    const placed = layoutRails(spans, 100);
    const lanes = placed.map((rail) => rail.lane);
    expect(Math.max(...lanes)).toBeLessThanOrEqual(maxRailLanes(100) - 1);
    expect(lanes.at(-1)).toBe(Math.max(...lanes));
  });

  it('짧은 막대도 최소 높이를 갖는다', () => {
    const [rail] = layoutRails([span('a:0', 0, 2)], 100);
    expect(rail.height).toBeGreaterThanOrEqual(14);
  });

  it('막대는 위에서 아래 순으로 정렬된다', () => {
    const placed = layoutRails([span('b:0', 80, 10), span('a:0', 10, 10)], 100);
    expect(placed.map((rail) => rail.id)).toEqual(['a:0', 'b:0']);
  });

  it('칩은 막대와 겹치지 않는 자리에 놓인다', () => {
    const placed = layoutRails([span('a:0', 0, 60), span('b:0', 5, 60)], 100);
    const [first, second] = placed;
    const overlaps =
      first.chipLeft < second.chipLeft + RAIL_CHIP_SIZE &&
      second.chipLeft < first.chipLeft + RAIL_CHIP_SIZE &&
      first.chipTop < second.chipTop + RAIL_CHIP_SIZE &&
      second.chipTop < first.chipTop + RAIL_CHIP_SIZE;
    expect(overlaps).toBe(false);
    expect(intersects(chipBox(first), barBox(second))).toBe(false);
    expect(intersects(chipBox(second), barBox(first))).toBe(false);
  });

  // 소비자는 거터가 좁으면 이격을 그만큼 줄여 부른다 — 그때도 0번 레인은 컨테이너 안에 남아야 한다.
  // 막대가 안 보이는 것은 좁은 막대보다 나쁘다(팝오버 모드가 상시 이 구간이다).
  it('좁은 거터에서도 0번 레인의 막대와 칩이 음수로 나가지 않는다', () => {
    for (const gutter of [20, 30, 40, 47, 60, 100]) {
      const gap = Math.min(RAIL_TEXT_GAP, Math.max(0, gutter - RAIL_WIDTH));
      const [rail] = layoutRails([span('a:0', 0, 20)], gutter, gap);
      expect(rail.lane).toBe(0);
      expect(rail.left).toBeGreaterThanOrEqual(0);
      expect(rail.chipLeft).toBeGreaterThanOrEqual(0);
      // 칩은 layoutRails 안에서 배치되므로 이 단언이 gap이 실제로 흘러 들어갔는지를 잡는다 —
      // gap을 무시하면 막대를 음수에 두고 칩을 0에 얹어 둘이 겹친다
      expect(intersects(chipBox(rail), barBox(rail))).toBe(false);
    }
  });
});

describe('maxRailLanes', () => {
  it('거터 100은 여섯 레인이다', () => {
    expect(maxRailLanes(100)).toBe(6);
  });

  it('거터가 좁아지면 레인도 줄어든다', () => {
    expect(maxRailLanes(79)).toBe(3);
    expect(maxRailLanes(79)).toBeLessThan(maxRailLanes(100));
  });

  it('셈이 0 이하로 떨어져도 한 레인은 남는다', () => {
    expect(maxRailLanes(64)).toBe(1);
    expect(maxRailLanes(0)).toBe(1);
  });
});

describe('layoutRailHitTargets', () => {
  it('세로로 겹치지 않는 rail 덩어리는 각각 본문과 최소 간격을 두고 우측 정렬한다', () => {
    const targets = layoutRailHitTargets([span('first:0', 0, 20), span('second:0', 40, 20)], 100);

    expect(targets.map((target) => target.hitBox.right)).toEqual([84, 84]);
  });

  it('숫자와 막대를 포함하는 가장 작은 사각형을 만든다', () => {
    const [target] = layoutRailHitTargets([span('a:0', 10, 40)], 100);

    expect(target.hitBox).toEqual({ left: 63, top: 10, right: 84, bottom: 50 });
  });

  it('같은 레인에서는 작은 클릭 범위에 더 높은 우선순위를 준다', () => {
    const [large, small] = layoutRailHitTargets([span('large:0', 0, 80), span('small:0', 100, 20)], 100);

    expect(small.lane).toBe(large.lane);
    expect(small.hitPriority).toBeGreaterThan(large.hitPriority);
  });

  it('클릭 범위가 더 커도 바깥 레인에 놓인 것에 높은 우선순위를 준다', () => {
    const [inner, outer] = layoutRailHitTargets([span('inner:0', 0, 20), span('outer:0', 5, 80)], 100);

    expect(outer.lane).toBeGreaterThan(inner.lane);
    expect((outer.hitBox.right - outer.hitBox.left) * (outer.hitBox.bottom - outer.hitBox.top)).toBeGreaterThan(
      (inner.hitBox.right - inner.hitBox.left) * (inner.hitBox.bottom - inner.hitBox.top),
    );
    expect(outer.hitPriority).toBeGreaterThan(inner.hitPriority);
  });
});

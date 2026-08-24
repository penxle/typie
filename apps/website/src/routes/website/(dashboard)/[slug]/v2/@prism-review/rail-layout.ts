export type RailTone = 'open' | 'closed' | 'strength';

export type RailSpan = {
  id: string;
  itemId: string;
  number: number;
  tone: RailTone;
  top: number;
  height: number;
};

export type PlacedRail = RailSpan & { lane: number; chipTop: number; chipLeft: number };

export type RailHitBox = { left: number; top: number; right: number; bottom: number };
export type RailHitTarget = PlacedRail & { hitBox: RailHitBox; hitPriority: number };

export const RAIL_WIDTH = 3;
export const RAIL_CHIP_SIZE = 16;
export const RAIL_TEXT_GAP = 44;

const RAIL_LANE_STEP = 7;
const RAIL_MIN_HEIGHT = 14;
const RAIL_CHIP_GAP = 2;
const LANE_CLEARANCE = 6;

// 레인 상한의 구속 조건은 막대가 아니라 왼쪽 칩 자리다 — 마지막 레인도 왼쪽 배치가 가능해야
// 깊은 레인의 칩이 중앙 폴백으로 밀리지 않는다.
export const maxRailLanes = (gutter: number): number =>
  Math.max(1, Math.floor((gutter - RAIL_TEXT_GAP - RAIL_WIDTH - RAIL_CHIP_SIZE - RAIL_CHIP_GAP) / RAIL_LANE_STEP) + 1);

// 공간이 모자라면 양보하는 것은 막대가 아니라 본문과의 이격이다 — 거터가 좁을 때 gap을 줄여 부르면
// 0번 레인이 컨테이너 왼쪽 밖으로 나가지 않는다. maxRailLanes는 gap을 받지 않아도 된다:
// gap이 줄어드는 구간(거터 < 47)에서는 어느 쪽 셈이든 결과가 1레인으로 같다.
export const railLeft = (lane: number, gutter: number, gap = RAIL_TEXT_GAP): number => gutter - gap - RAIL_WIDTH - lane * RAIL_LANE_STEP;

const intersects = (a: RailHitBox, b: RailHitBox) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

export const layoutRails = (spans: readonly RailSpan[], gutter: number, gap = RAIL_TEXT_GAP): PlacedRail[] => {
  const lanes = maxRailLanes(gutter);
  const rails: PlacedRail[] = spans
    .map((span) => ({ ...span, height: Math.max(RAIL_MIN_HEIGHT, span.height), lane: 0, chipTop: span.top, chipLeft: 0 }))
    .toSorted((a, b) => a.top - b.top);

  const laneBottoms: number[] = [];
  for (const rail of rails) {
    let lane = laneBottoms.findIndex((bottom) => bottom + LANE_CLEARANCE <= rail.top);
    if (lane === -1) {
      // 거터 밖(본문·페이지 여백)으로 탈출하는 것보다 겹침이 낫다
      if (laneBottoms.length < lanes) {
        lane = laneBottoms.length;
        laneBottoms.push(0);
      } else {
        lane = lanes - 1;
      }
    }
    laneBottoms[lane] = Math.max(laneBottoms[lane] ?? 0, rail.top + rail.height);
    rail.lane = lane;
  }

  const bars: RailHitBox[] = rails.map((rail) => {
    const left = railLeft(rail.lane, gutter, gap);
    return { left, top: rail.top, right: left + RAIL_WIDTH, bottom: rail.top + rail.height };
  });

  const chips: RailHitBox[] = [];
  for (const [index, rail] of rails.entries()) {
    const barX = railLeft(rail.lane, gutter, gap);
    const sides = [barX - RAIL_CHIP_SIZE - RAIL_CHIP_GAP, barX + RAIL_WIDTH + RAIL_CHIP_GAP];
    let chosen: RailHitBox | null = null;

    for (const left of sides) {
      if (left < 0) continue;
      const box = { left, top: rail.top, right: left + RAIL_CHIP_SIZE, bottom: rail.top + RAIL_CHIP_SIZE };
      const blocked = bars.some((bar, at) => at !== index && intersects(box, bar)) || chips.some((chip) => intersects(box, chip));
      if (!blocked) {
        chosen = box;
        break;
      }
    }

    if (!chosen) {
      // 양쪽 다 막힘 — 막대 위 중앙에 얹고, 칩끼리 겹치면 아래로 민다
      const left = Math.max(0, barX + RAIL_WIDTH / 2 - RAIL_CHIP_SIZE / 2);
      let top = rail.top;
      for (;;) {
        const box = { left, top, right: left + RAIL_CHIP_SIZE, bottom: top + RAIL_CHIP_SIZE };
        if (chips.every((chip) => !intersects(box, chip))) {
          chosen = box;
          break;
        }
        top += RAIL_CHIP_SIZE + RAIL_CHIP_GAP;
      }
    }

    chips.push(chosen);
    rail.chipLeft = chosen.left;
    rail.chipTop = chosen.top;
  }

  return rails;
};

export const layoutRailHitTargets = (spans: readonly RailSpan[], gutter: number, gap = RAIL_TEXT_GAP): RailHitTarget[] => {
  const targets = layoutRails(spans, gutter, gap).map((rail) => {
    const barLeft = railLeft(rail.lane, gutter, gap);
    return {
      ...rail,
      hitBox: {
        left: Math.min(barLeft, rail.chipLeft),
        top: Math.min(rail.top, rail.chipTop),
        right: Math.max(barLeft + RAIL_WIDTH, rail.chipLeft + RAIL_CHIP_SIZE),
        bottom: Math.max(rail.top + rail.height, rail.chipTop + RAIL_CHIP_SIZE),
      },
    };
  });
  const area = (target: (typeof targets)[number]) =>
    (target.hitBox.right - target.hitBox.left) * (target.hitBox.bottom - target.hitBox.top);
  const ordered = targets.toSorted((a, b) => a.lane - b.lane || area(b) - area(a));
  const priority = new Map(ordered.map((target, hitPriority) => [target, hitPriority]));

  return targets.map((target) => ({ ...target, hitPriority: priority.get(target) ?? 0 }));
};

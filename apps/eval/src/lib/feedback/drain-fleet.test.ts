import { describe, expect, it } from 'vitest';
import { DrainFleet, LINGER_MS } from './drain-fleet.ts';

const line = (id: number, text: string) => ({ id, text, stage: 'description' as const, round: null });

// 16ms 프레임으로 from~to(ms)를 결정적으로 돌린다.
const run = (fleet: DrainFleet, from: number, to: number) => {
  for (let now = from + 16; now <= to; now += 16) fleet.advance(now, 16);
};

describe('drain fleet', () => {
  it('라이브의 연장선(prefix)일 때만 넘겨받고, 같은 라인은 한 번만 등재한다', () => {
    const fleet = new DrainFleet();
    expect(fleet.seal(line(1, '문장의 전문입니다'), '문장의 전', 5, 15, 0)).toBe(true);
    expect(fleet.seal(line(1, '문장의 전문입니다'), '문장의 전', 5, 15, 0)).toBe(false); // 중복
    expect(fleet.seal(line(2, '전혀 다른 문장'), '문장의 전', 0, 15, 0)).toBe(false); // 대조 실패
    expect(fleet.seal(line(3, '무엇이든'), '', 0, 15, 0)).toBe(false); // 라이브가 비어 있었다
    expect(fleet.slots).toHaveLength(1);
  });

  it('꼬리를 흘린 뒤에도 링거 동안 남고, 링거가 지나야 걷힌다', () => {
    const fleet = new DrainFleet();
    fleet.seal(line(1, '짧은 꼬리'), '짧은', 2, 15, 0);
    let now = 0;
    while (fleet.slots[0]?.doneAt === null && now < 5000) {
      now += 16;
      fleet.advance(now, 16);
    }
    const doneAt = fleet.slots[0]?.doneAt ?? null;
    expect(doneAt).not.toBeNull(); // 꼬리를 다 흘렸다
    fleet.advance((doneAt ?? 0) + LINGER_MS - 100, 16);
    expect(fleet.active).toBe(true); // 링거 중 — 카드는 아직 접히지 않는다
    fleet.advance((doneAt ?? 0) + LINGER_MS + 100, 16);
    expect(fleet.active).toBe(false); // 링거 만료 — 이제 접혀도 된다
  });

  it('이미 전부 보인 라인도 링거로 등재된다 — 꼬리 없는 마무리에도 읽을 시간을 준다', () => {
    const fleet = new DrainFleet();
    fleet.seal(line(1, '전부 보임'), '전부 보임', 5, 15, 1000);
    expect(fleet.active).toBe(true);
    expect(fleet.slots[0].doneAt).toBe(1000);
    run(fleet, 1000, 1000 + LINGER_MS - 100);
    expect(fleet.active).toBe(true);
    run(fleet, 1000 + LINGER_MS - 100, 1000 + LINGER_MS + 200);
    expect(fleet.active).toBe(false);
  });

  it('여러 슬롯이 각자 완주한다 — 뒤 봉인이 앞 꼬리를 덮지 않는다', () => {
    const fleet = new DrainFleet();
    fleet.seal(line(1, '가'.repeat(400)), '가'.repeat(10), 10, 15, 0); // 긴 꼬리
    run(fleet, 0, 200);
    fleet.seal(line(2, '나나나 짧은 오프너'), '나나나', 3, 15, 200); // 드레인 중 다음 봉인
    expect(fleet.slots).toHaveLength(2);
    run(fleet, 200, 10_000);
    expect(fleet.active).toBe(false); // 둘 다 완주+링거 만료
  });
});

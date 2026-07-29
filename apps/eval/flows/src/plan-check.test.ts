import { describe, expect, it } from 'vitest';
import { checkPlan } from './plan-check.ts';
import type { Plan } from './analysis-types.ts';

const CONTENT = '홍길동은 천천히 문을 열었다. 김영희가 손을 들어 홍길동을 불렀다. 순간 아무 소리도 없었다.';

const plan = (over: Partial<Plan>): Plan => ({
  intent: '테스트',
  protected: [],
  rejectedFindings: [],
  axes: [
    { label: '축1', description: 'a', risk: 'r', evidence: ['문을 열었다'] },
    { label: '축2', description: 'a', risk: 'r', evidence: ['홍길동을 불렀다'] },
    { label: '축3', description: 'a', risk: 'r', evidence: ['소리도 없었다'] },
  ],
  ...over,
});

describe('checkPlan', () => {
  it('실재하지 않는 인용을 걷어내고 기록한다', () => {
    const result = checkPlan(
      CONTENT,
      plan({ protected: [{ technique: '기법', evidence: ['문을 열었다', '원고에 없는 문장'], rationale: 'r' }] }),
    );
    expect(result.plan.protected[0].evidence).toEqual(['문을 열었다']);
    expect(result.notes.some((n) => n.includes('원고에 없음'))).toBe(true);
  });

  // 근거가 전부 사라진 보호는 사면장이다 — 등재 자체를 취소한다.
  it('근거가 전부 미실재인 보호는 등재를 취소한다', () => {
    const result = checkPlan(CONTENT, plan({ protected: [{ technique: '허구', evidence: ['없는 문장'], rationale: 'r' }] }));
    expect(result.plan.protected).toEqual([]);
    expect(result.notes.some((n) => n.includes('등재 취소'))).toBe(true);
  });

  // 축은 근거가 비어도 제거하지 않는다 — 제거하면 축 수 계약이 조용히 깨진다.
  it('근거 없는 축은 남기되 기록한다', () => {
    const result = checkPlan(CONTENT, plan({ axes: plan({}).axes.map((a, i) => (i === 0 ? { ...a, evidence: ['없는 문장'] } : a)) }));
    expect(result.plan.axes).toHaveLength(3);
    expect(result.plan.axes[0].evidence).toEqual([]);
    expect(result.axisCountOk).toBe(true);
    expect(result.notes.some((n) => n.includes('실재하는 근거가 없음'))).toBe(true);
  });

  it('축 수 계약 위반을 판정한다', () => {
    const one = checkPlan(CONTENT, plan({ axes: plan({}).axes.slice(0, 1) }));
    expect(one.axisCountOk).toBe(false);
    expect(one.notes.some((n) => n.includes('계약'))).toBe(true);
    expect(checkPlan(CONTENT, plan({})).axisCountOk).toBe(true);
  });

  it('보호 상한을 자른다', () => {
    const many = Array.from({ length: 10 }, (_, i) => ({ technique: `기법${i}`, evidence: ['문을 열었다'], rationale: 'r' }));
    const result = checkPlan(CONTENT, plan({ protected: many }));
    expect(result.plan.protected).toHaveLength(8);
    expect(result.notes.some((n) => n.includes('상한'))).toBe(true);
  });
});

import { describe, expect, it } from 'vitest';
import { nestedRound, STAGES, stagesFor, stepStage, TERMINAL_EVENTS, TIER_STAGES } from './stages.ts';
import { TIER_AGENTS, TIER_NAMES } from './tiers.ts';

describe('stage mapping', () => {
  it('여섯 스테이지의 순서와 라벨이 고정된다', () => {
    expect(STAGES.map((s) => s.label)).toEqual([
      '원고 살펴보기',
      '계획 세우기',
      '짚을 곳 찾기',
      '문장 살피기',
      '전할 말 고르기',
      '마무리 글 쓰기',
    ]);
  });

  it('step 이름 접두를 표시 스테이지로 매핑한다', () => {
    expect(stepStage('manuscript')).toBeNull();
    expect(stepStage('research-0')).toBe('research');
    expect(stepStage('plan-1')).toBe('plan');
    expect(stepStage('audit-1')).toBe('plan');
    expect(stepStage('plan-review-1-0')).toBe('plan');
    expect(stepStage('plan-revise-1-0')).toBe('plan');
    expect(stepStage('critique-2')).toBe('critique');
    expect(stepStage('proofread-0')).toBe('proofread');
    expect(stepStage('remarks')).toBe('rephrase');
    expect(stepStage('rephrase-0')).toBe('rephrase');
    expect(stepStage('tally')).toBe('conclude');
    expect(stepStage('conclude-1')).toBe('conclude');
  });

  it('계획 검수 스텝에서만 되짚기 회차를 읽는다', () => {
    expect(nestedRound('plan-review-1')).toBe(1);
    expect(nestedRound('plan-review-2-0')).toBe(2);
    expect(nestedRound('plan-0')).toBeNull();
    expect(nestedRound('plan-revise-1')).toBeNull();
    expect(nestedRound(null)).toBeNull();
  });

  it('터미널 이벤트 집합', () => {
    expect([...TERMINAL_EVENTS]).toEqual(['workflow.completed', 'workflow.failed', 'workflow.canceled']);
  });
});

describe('tier stages', () => {
  it('티어별 표시 스테이지가 고정된다', () => {
    expect(TIER_STAGES.high).toEqual(['research', 'plan', 'critique', 'proofread', 'rephrase', 'conclude']);
    expect(TIER_STAGES.medium).toEqual(['research', 'critique', 'proofread', 'rephrase', 'conclude']);
    expect(TIER_STAGES.low).toEqual(['critique', 'proofread', 'rephrase']);
  });

  it('stagesFor는 STAGES의 순서와 라벨을 유지한 채 걸러 낸다', () => {
    expect(stagesFor('high')).toEqual(STAGES);
    expect(stagesFor('medium').map((stage) => stage.key)).toEqual(['research', 'critique', 'proofread', 'rephrase', 'conclude']);
    expect(stagesFor('low')).toEqual([
      { key: 'critique', label: '짚을 곳 찾기' },
      { key: 'proofread', label: '문장 살피기' },
      { key: 'rephrase', label: '전할 말 고르기' },
    ]);
  });

  it('high는 STAGES 전체와 같고, 모든 티어는 STAGES의 부분열이다', () => {
    // 새 스테이지가 STAGES에 늘면 high가 먼저 깨진다 — 티어 표에 반영하라는 신호다.
    expect(TIER_STAGES.high).toEqual(STAGES.map((stage) => stage.key));
    for (const tier of TIER_NAMES) {
      // 부분열 = 순서를 지킨 채 걸러 낸 목록. stagesFor가 STAGES.filter라 이 등식이 곧 순서 보존의 증거다.
      expect(TIER_STAGES[tier]).toEqual(stagesFor(tier).map((stage) => stage.key));
    }
  });

  it('표시 스테이지와 티어 에이전트가 서로를 빠짐없이 덮는다', () => {
    // 중첩 검수자(review-high)는 plan 스테이지 안에서 도는 스텝이라 제 카드를 갖지 않는다 — 유일한 예외다.
    const NESTED = new Set(['review-high']);
    for (const tier of TIER_NAMES) {
      const agents = TIER_AGENTS[tier] as readonly string[];
      // 표시하는 스테이지에는 그 일을 하는 에이전트가 있다
      expect(TIER_STAGES[tier].map((stage) => `${stage}-${tier}`)).toEqual(agents.filter((agent) => !NESTED.has(agent)));
      // 에이전트가 있는 일은 빠짐없이 표시된다(중첩 제외)
      for (const agent of agents) {
        if (NESTED.has(agent)) continue;
        expect(TIER_STAGES[tier]).toContain(agent.slice(0, agent.length - tier.length - 1));
      }
    }
  });
});

import { describe, expect, it } from 'vitest';
import { agentStage, nestedRound, STAGES, stagesFor, stepStage, TERMINAL_EVENTS, TIER_STAGES } from './stages.ts';
import { TIER_NAMES } from './tiers.ts';

describe('stage mapping', () => {
  it('스테이지의 순서와 라벨이 고정된다 — 공통 선두 분류에 high 일곱과 medium·low 다섯', () => {
    expect(STAGES.map((s) => s.label)).toEqual([
      '원고 가늠하기',
      '작품 읽기',
      '작품 이해하기',
      '기준 세우기',
      '짚을 곳 찾기',
      '문장 살피기',
      '전할 말 정리하기',
      '원고 살펴보기',
      '짚을 곳 찾기',
      '문장 살피기',
      '전할 말 고르기',
      '마무리 글 쓰기',
    ]);
  });

  it('step 이름 접두를 표시 스테이지로 매핑한다', () => {
    expect(stepStage('manuscript')).toBeNull();
    expect(stepStage('meta')).toBeNull();
    // 전 티어 공통 선두 — 거부 판정이면 여기서 워크플로가 종결된다
    expect(stepStage('classify-0')).toBe('classify');
    expect(stepStage('classify-1')).toBe('classify');
    // high — 조사 기록과 수정 제출은 검수받는 쪽(기준 세우기)에 귀속된다
    expect(stepStage('description-0')).toBe('description');
    expect(stepStage('interpretation-0')).toBe('interpretation');
    expect(stepStage('rubric-0')).toBe('rubric');
    expect(stepStage('audit-1')).toBe('rubric');
    expect(stepStage('rubric-revise-1-0')).toBe('rubric');
    expect(stepStage('calibration-1-0')).toBe('rubric');
    expect(stepStage('judgment-0')).toBe('judgment');
    expect(stepStage('stylistic-0')).toBe('stylistic');
    expect(stepStage('findings')).toBe('delivery');
    expect(stepStage('delivery-0')).toBe('delivery');
    // medium·low
    expect(stepStage('research-0')).toBe('research');
    expect(stepStage('critique-2')).toBe('critique');
    expect(stepStage('proofread-0')).toBe('proofread');
    expect(stepStage('remarks')).toBe('rephrase');
    expect(stepStage('rephrase-0')).toBe('rephrase');
    expect(stepStage('tally')).toBe('conclude');
    expect(stepStage('conclude-1')).toBe('conclude');
  });

  it('에이전트 이름을 스테이지로 귀속한다', () => {
    expect(agentStage('classify')).toBe('classify');
    expect(agentStage('description-high')).toBe('description');
    expect(agentStage('rubric-high')).toBe('rubric');
    expect(agentStage('calibration-high')).toBe('rubric');
    expect(agentStage('judgment-high')).toBe('judgment');
    expect(agentStage('stylistic-high')).toBe('stylistic');
    expect(agentStage('delivery-high')).toBe('delivery');
    expect(agentStage('critique-low')).toBe('critique');
    expect(agentStage('proofread-medium')).toBe('proofread');
    expect(agentStage('rephrase-medium')).toBe('rephrase');
    expect(agentStage('manuscript')).toBeNull();
  });

  it('검수 스텝에서만 되짚기 회차를 읽는다', () => {
    expect(nestedRound('calibration-1')).toBe(1);
    expect(nestedRound('calibration-2-0')).toBe(2);
    expect(nestedRound('rubric-0')).toBeNull();
    expect(nestedRound('rubric-revise-1')).toBeNull();
    expect(nestedRound(null)).toBeNull();
  });

  it('터미널 이벤트 집합', () => {
    expect([...TERMINAL_EVENTS]).toEqual(['workflow.completed', 'workflow.failed', 'workflow.canceled']);
  });
});

describe('tier stages', () => {
  it('티어별 표시 스테이지가 고정된다', () => {
    expect(TIER_STAGES.high).toEqual(['classify', 'description', 'interpretation', 'rubric', 'judgment', 'stylistic', 'delivery']);
    expect(TIER_STAGES.medium).toEqual(['classify', 'research', 'critique', 'proofread', 'rephrase', 'conclude']);
    expect(TIER_STAGES.low).toEqual(['classify', 'critique', 'proofread', 'rephrase']);
  });

  it('stagesFor는 STAGES의 순서와 라벨을 유지한 채 걸러 낸다', () => {
    expect(stagesFor('high').map((stage) => stage.key)).toEqual(TIER_STAGES.high);
    expect(stagesFor('medium').map((stage) => stage.key)).toEqual([
      'classify',
      'research',
      'critique',
      'proofread',
      'rephrase',
      'conclude',
    ]);
    expect(stagesFor('low')).toEqual([
      { key: 'classify', label: '원고 가늠하기' },
      { key: 'critique', label: '짚을 곳 찾기' },
      { key: 'proofread', label: '문장 살피기' },
      { key: 'rephrase', label: '전할 말 고르기' },
    ]);
  });

  it('STAGES는 티어 표의 합집합이고, 모든 티어는 STAGES의 부분열이다', () => {
    // 어느 티어에도 없는 스테이지가 STAGES에 남아 있으면 화면에 죽은 카드가 선다.
    expect(new Set(STAGES.map((stage) => stage.key))).toEqual(new Set(TIER_NAMES.flatMap((tier) => TIER_STAGES[tier])));
    for (const tier of TIER_NAMES) {
      // 부분열 = 순서를 지킨 채 걸러 낸 목록. stagesFor가 STAGES.filter라 이 등식이 곧 순서 보존의 증거다.
      expect(TIER_STAGES[tier]).toEqual(stagesFor(tier).map((stage) => stage.key));
    }
  });
});

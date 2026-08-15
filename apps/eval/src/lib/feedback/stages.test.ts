import { describe, expect, it } from 'vitest';
import { agentStage, nestedRound, STAGES, stagesFor, stepStage, TERMINAL_EVENTS, TIER_STAGES } from './stages.ts';
import { TIER_NAMES } from './tiers.ts';

describe('stage mapping', () => {
  it('스테이지의 순서와 라벨이 고정된다 — 공통 선두 분류에 high 여섯(medium·low는 그 부분열)', () => {
    expect(STAGES.map((s) => s.label)).toEqual([
      '원고 가늠하기',
      '작품 읽기',
      '작품 이해하기',
      '기준 세우기',
      '짚을 곳 찾기',
      '문장 살피기',
      '전할 말 정리하기',
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
    // 판정·문면·전달 스텝은 전 티어가 이름을 공유한다. 프로그램 스텝(전처리·처분·연속성)은 어느 단계에도 귀속되지 않는다
    expect(stepStage('prepare')).toBeNull();
    expect(stepStage('previous-threads')).toBeNull();
    expect(stepStage('dispositions')).toBeNull();
    expect(stepStage('continuity')).toBeNull();
    // 구 medium 구성(research→conclude)은 철거됐다 — 그 구성으로 굳은 세션의 스텝은 어느 단계에도 서지 않는다
    expect(stepStage('research-0')).toBeNull();
    expect(stepStage('critique-2')).toBeNull();
    expect(stepStage('conclude-1')).toBeNull();
  });

  it('에이전트 이름을 스테이지로 귀속한다', () => {
    expect(agentStage('classify')).toBe('classify');
    expect(agentStage('description-high')).toBe('description');
    expect(agentStage('rubric-high')).toBe('rubric');
    expect(agentStage('calibration-high')).toBe('rubric');
    expect(agentStage('judgment-high')).toBe('judgment');
    expect(agentStage('stylistic-high')).toBe('stylistic');
    expect(agentStage('delivery-high')).toBe('delivery');
    expect(agentStage('judgment-low')).toBe('judgment');
    expect(agentStage('stylistic-low')).toBe('stylistic');
    expect(agentStage('delivery-low-followup')).toBe('delivery');
    expect(agentStage('description-medium')).toBe('description');
    expect(agentStage('judgment-medium-followup')).toBe('judgment');
    expect(agentStage('proofread-medium')).toBeNull();
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
    expect(TIER_STAGES.medium).toEqual(['classify', 'description', 'judgment', 'stylistic', 'delivery']);
    expect(TIER_STAGES.low).toEqual(['classify', 'judgment', 'stylistic', 'delivery']);
  });

  it('stagesFor는 STAGES의 순서와 라벨을 유지한 채 걸러 낸다', () => {
    expect(stagesFor('high').map((stage) => stage.key)).toEqual(TIER_STAGES.high);
    expect(stagesFor('medium').map((stage) => stage.key)).toEqual(['classify', 'description', 'judgment', 'stylistic', 'delivery']);
    expect(stagesFor('low')).toEqual([
      { key: 'classify', label: '원고 가늠하기' },
      { key: 'judgment', label: '짚을 곳 찾기' },
      { key: 'stylistic', label: '문장 살피기' },
      { key: 'delivery', label: '전할 말 정리하기' },
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

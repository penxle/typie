import { describe, expect, it } from 'vitest';
import { nestedRound, STAGES, stepStage, TERMINAL_EVENTS } from './stages.ts';

describe('stage mapping', () => {
  it('여섯 스테이지의 순서와 라벨이 고정된다', () => {
    expect(STAGES.map((s) => s.label)).toEqual(['원고 살펴보기', '계획 세우기', '작품 읽기', '문장 살피기', '피드백 다듬기', '총평 쓰기']);
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
    expect([...TERMINAL_EVENTS]).toEqual(['run.completed', 'run.failed', 'run.canceled']);
  });
});

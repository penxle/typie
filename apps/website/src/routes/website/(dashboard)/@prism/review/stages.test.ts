import { describe, expect, it } from 'vitest';
import { stagesFor, stepRound, stepStage } from './stages.ts';

describe('stages', () => {
  it('티어별 단계 순서', () => {
    expect(stagesFor('low').map((s) => s.key)).toEqual(['classify', 'judgment', 'stylistic', 'delivery']);
    expect(stagesFor('medium').map((s) => s.key)).toEqual(['classify', 'description', 'judgment', 'stylistic', 'delivery']);
    expect(stagesFor('high').map((s) => s.key)).toEqual([
      'classify',
      'description',
      'interpretation',
      'rubric',
      'judgment',
      'stylistic',
      'delivery',
    ]);
  });

  it('스텝 이름 접두로 단계를 찾고, 준비·정산 스텝은 단계가 없다', () => {
    expect(stepStage('classify-0')).toBe('classify');
    expect(stepStage('description-1')).toBe('description');
    expect(stepStage('audit-2')).toBe('rubric');
    expect(stepStage('calibration-1')).toBe('rubric');
    expect(stepStage('calibration-1-0')).toBe('rubric');
    expect(stepStage('rubric-revise')).toBe('rubric');
    expect(stepStage('rubric-revise-1-0')).toBe('rubric');
    expect(stepStage('interpretation-0')).toBe('interpretation');
    expect(stepStage('stylistic-1')).toBe('stylistic');
    expect(stepStage('delivery-0')).toBe('delivery');
    expect(stepStage('judgment-targets')).toBeNull();
    expect(stepStage('stylistic-targets')).toBeNull();
    expect(stepStage('manuscript')).toBeNull();
    expect(stepStage('findings')).toBeNull();
    expect(stepStage('meta')).toBeNull();
    expect(stepStage('prepare')).toBeNull();
    expect(stepStage('previous-threads')).toBeNull();
    expect(stepStage('dispositions')).toBeNull();
    expect(stepStage('continuity')).toBeNull();
  });

  it('점검 라운드 번호 — 제출 시도 접미가 붙어도 라운드는 같다', () => {
    expect(stepRound('audit-1')).toBe(1);
    expect(stepRound('calibration-1-0')).toBe(1);
    expect(stepRound('calibration-3')).toBe(3);
    expect(stepRound('calibration-3-2')).toBe(3);
    expect(stepRound('rubric-revise-2-1')).toBeNull();
    expect(stepRound('rubric-revise')).toBeNull();
    expect(stepRound('rubric-0')).toBeNull();
    expect(stepRound('judgment-0')).toBeNull();
  });
});

import { describe, expect, it } from 'vitest';
import { ACCIDENT_AXIS, REVIEW_TOOL, reviewToolWithAxes } from './analysis-tools.ts';

type FindingsSchema = { properties: { findings: { items: { properties: Record<string, { enum?: string[] }>; required: string[] } } } };

describe('reviewToolWithAxes', () => {
  it('문서별 축을 enum으로 강제하고 고정 축을 항상 포함한다', () => {
    const tool = reviewToolWithAxes(['무전 화자 표지', '후반부 시점']);
    const items = (tool.input_schema as unknown as FindingsSchema).properties.findings.items;
    expect(items.properties.axis.enum).toEqual(['무전 화자 표지', '후반부 시점', ACCIDENT_AXIS]);
    expect(items.required).toContain('axis');
  });

  // 빈 enum은 스키마로 성립하지 않는다 — 조용한 무계획 폴백 대신 문서를 세운다.
  it('축이 비면 던진다', () => {
    expect(() => reviewToolWithAxes([])).toThrow();
  });

  it('원본 도구는 손대지 않는다', () => {
    const items = (REVIEW_TOOL.input_schema as unknown as FindingsSchema).properties.findings.items;
    expect(items.properties.axis).toBeUndefined();
    expect(items.required).not.toContain('axis');
  });
});

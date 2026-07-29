import { describe, expect, it } from 'vitest';
import { schemaViolations } from './tool-schema.ts';

const SURVEY_SCHEMA = {
  type: 'object',
  properties: {
    form: { type: 'string' },
    isDerivative: { type: 'boolean' },
    deliberateStyles: {
      type: 'array',
      items: {
        type: 'object',
        properties: { pattern: { type: 'string' }, evidence: { type: 'string' } },
        required: ['pattern', 'evidence'],
      },
    },
    scenes: {
      type: 'array',
      items: {
        type: 'object',
        properties: { startQuote: { type: 'string' }, endQuote: { type: 'string' } },
        required: ['startQuote', 'endQuote'],
      },
    },
  },
  required: ['form', 'isDerivative', 'deliberateStyles', 'scenes'],
};

const valid = {
  form: '단편소설',
  isDerivative: false,
  deliberateStyles: [{ pattern: '가', evidence: '나' }],
  scenes: [{ startQuote: '다', endQuote: '라' }],
};

describe('schemaViolations', () => {
  it('어긴 것이 없으면 빈 배열', () => {
    expect(schemaViolations(SURVEY_SCHEMA, valid)).toEqual([]);
  });

  it('필수 필드 누락을 잡는다', () => {
    const rest = Object.fromEntries(Object.entries(valid).filter(([key]) => key !== 'form'));
    expect(schemaViolations(SURVEY_SCHEMA, rest)).toEqual(['form: 빠졌습니다']);
  });

  // 실제로 문서 3편을 실패시킨 경우 — 배열이어야 할 값이 JSON 문자열로 왔다.
  it('배열 자리에 문자열이 오면 잡는다', () => {
    const broken = { ...valid, deliberateStyles: '[{"pattern":"가","evidence":"나"}]' };
    expect(schemaViolations(SURVEY_SCHEMA, broken)).toEqual(['deliberateStyles: array가 와야 하는데 string입니다']);
  });

  it('배열 원소 안의 누락을 인덱스와 함께 잡는다', () => {
    const broken = { ...valid, scenes: [{ startQuote: '다', endQuote: '라' }, { startQuote: '마' }] };
    expect(schemaViolations(SURVEY_SCHEMA, broken)).toEqual(['scenes[1].endQuote: 빠졌습니다']);
  });

  it('배열 원소의 타입 위반도 인덱스와 함께 잡는다', () => {
    const broken = { ...valid, deliberateStyles: [{ pattern: 1, evidence: '나' }] };
    expect(schemaViolations(SURVEY_SCHEMA, broken)).toEqual(['deliberateStyles[0].pattern: string가 와야 하는데 number입니다']);
  });

  it('불리언 자리에 문자열이 오면 잡는다', () => {
    expect(schemaViolations(SURVEY_SCHEMA, { ...valid, isDerivative: 'false' })).toEqual([
      'isDerivative: boolean가 와야 하는데 string입니다',
    ]);
  });

  it('null은 값으로 보되 타입은 따진다', () => {
    expect(schemaViolations({ type: 'object', required: ['a'] }, { a: null })).toEqual([]);
    expect(schemaViolations({ type: 'object', properties: { a: { type: 'array' } } }, { a: null })).toEqual([
      'a: array가 와야 하는데 null입니다',
    ]);
  });

  it('빈 배열은 위반이 아니다', () => {
    expect(schemaViolations(SURVEY_SCHEMA, { ...valid, deliberateStyles: [] })).toEqual([]);
  });

  it('스키마가 비었으면 아무것도 요구하지 않는다', () => {
    expect(schemaViolations(undefined, { anything: 1 })).toEqual([]);
  });
});

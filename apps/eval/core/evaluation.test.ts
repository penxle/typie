import { describe, expect, it } from 'vitest';
import { isJudgmentComplete, judgmentGaps, missingFields, sanitizePayload, targetFor } from './evaluation.ts';
import type { EvaluationSpec, FieldSpec } from './contracts.ts';

// 코어는 필드의 종류를 모른다. 테스트도 자기 필드를 직접 만들어 그 사실을 고정한다.
const bool = (key: string): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (raw === true || raw === false ? raw : null),
  render: { kind: 'bool' },
});

const optionalText = (key: string, maxLength = 1000): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => (typeof raw === 'string' && raw.trim() ? raw.trim().slice(0, maxLength) : null),
  render: { kind: 'text' },
});

const ranged = (key: string, min: number, max: number): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (Number.isSafeInteger(raw) && (raw as number) >= min && (raw as number) <= max ? raw : null),
  render: { kind: 'ranged' },
});

// 코어가 모르는 위젯도 그대로 통과한다는 것이 이 설계의 요점이다.
const labels = (key: string, options: string[]): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => {
    if (!Array.isArray(raw)) return null;
    const kept = raw.filter((v) => typeof v === 'string' && options.includes(v));
    return kept.length > 0 ? kept : null;
  },
  render: { kind: 'labels', options },
});

const stage = {
  run: [ranged('helpfulness', 1, 5), optionalText('comment')],
  items: [{ match: (item: { kind: string }) => item.kind === 'finding', fields: [bool('correct'), optionalText('note')] }],
};

const evaluation: EvaluationSpec = {
  id: 'sample',
  label: '표본',
  stages: [
    { key: 'judgment', label: '판정', ...stage },
    {
      key: 'extra',
      label: '추가',
      run: [bool('extraOk')],
      items: [{ match: (item) => item.kind === 'strength', fields: [bool('agree')] }],
    },
  ],
};

const finding = { id: 'i1', kind: 'finding', facets: {} };
const strength = { id: 'i2', kind: 'strength', facets: {} };
const pattern = { id: 'i3', kind: 'pattern', facets: {} };

describe('targetFor', () => {
  it('단계를 가로질러 match되는 대상을 찾는다', () => {
    expect(targetFor(evaluation, finding)?.fields).toHaveLength(2);
    expect(targetFor(evaluation, strength)?.fields).toHaveLength(1);
  });

  it('match되지 않으면 null', () => {
    expect(targetFor(evaluation, pattern)).toBeNull();
  });
});

describe('missingFields', () => {
  it('required가 아닌 필드는 요구하지 않는다', () => {
    expect(missingFields(stage.items[0].fields, { correct: true })).toEqual([]);
  });

  it('null은 미답이다', () => {
    expect(missingFields(stage.items[0].fields, { correct: null })).toEqual(['correct']);
  });

  it('false는 답이다', () => {
    expect(missingFields(stage.items[0].fields, { correct: false })).toEqual([]);
  });

  it('sanitize가 null을 내면 미답이다', () => {
    expect(missingFields(stage.run, { helpfulness: 0 })).toEqual(['helpfulness']);
    expect(missingFields(stage.run, { helpfulness: 1 })).toEqual([]);
  });

  it('코어가 모르는 위젯도 같은 규칙으로 판정한다', () => {
    const fields = [labels('tags', ['a', 'b'])];
    expect(missingFields(fields, { tags: [] })).toEqual(['tags']);
    expect(missingFields(fields, { tags: ['a', 'z'] })).toEqual([]);
  });
});

describe('judgmentGaps', () => {
  it('대상이 아닌 항목은 요구하지 않는다', () => {
    expect(judgmentGaps(stage, [finding, strength], { helpfulness: 4 }, { i1: { correct: true } })).toEqual({ run: [], items: [] });
  });

  it('빠진 항목과 빠진 필드를 함께 보고한다', () => {
    const gaps = judgmentGaps(stage, [finding], {}, {});
    expect(gaps.run).toEqual(['helpfulness']);
    expect(gaps.items).toEqual([{ itemId: 'i1', missing: ['correct'] }]);
  });
});

describe('isJudgmentComplete', () => {
  it('빈틈이 없으면 참', () => {
    expect(isJudgmentComplete(stage, [finding], { helpfulness: 5 }, { i1: { correct: false } })).toBe(true);
  });

  it('하나라도 비면 거짓', () => {
    expect(isJudgmentComplete(stage, [finding], { helpfulness: 5 }, {})).toBe(false);
  });
});

describe('sanitizePayload', () => {
  it('선언에 없는 키를 버린다', () => {
    expect(sanitizePayload(stage.items[0].fields, { correct: true, sneaky: 1 })).toEqual({ correct: true, note: null });
  });

  it('형이 다른 값은 null로 떨어뜨린다', () => {
    expect(sanitizePayload(stage.items[0].fields, { correct: 'yes' })).toEqual({ correct: null, note: null });
  });

  it('공백만인 텍스트는 null이다', () => {
    expect(sanitizePayload(stage.run, { comment: ' '.repeat(3) })).toEqual({ helpfulness: null, comment: null });
  });

  it('텍스트를 최대 길이로 자른다', () => {
    expect(sanitizePayload([optionalText('note', 3)], { note: 'abcdef' })).toEqual({ note: 'abc' });
  });

  it('범위 밖 값은 null이다', () => {
    expect(sanitizePayload(stage.run, { helpfulness: 9 })).toEqual({ helpfulness: null, comment: null });
    expect(sanitizePayload(stage.run, { helpfulness: 3 })).toEqual({ helpfulness: 3, comment: null });
  });
});

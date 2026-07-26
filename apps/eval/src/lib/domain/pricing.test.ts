import { describe, expect, it } from 'vitest';
import { costPerCharacter, DEFAULT_PRICE_TABLE, estimateCost, formatKrw, parsePriceTable } from './pricing.ts';

const table = { models: { m1: { input: 5, output: 25, cacheRead: 0.5 }, m2: { input: 1, output: 5 } }, usdKrw: 1000 };

describe('estimateCost', () => {
  it('단일 모델이면 입력·출력을 각각 단가에 곱한다', () => {
    const cost = estimateCost({ promptTokens: 1_000_000, completionTokens: 200_000, models: ['m1'] }, table);
    expect(cost).toEqual({ kind: 'exact', model: 'm1', usd: 10, krw: 10_000 });
  });

  // 캐시 몫은 promptTokens 안에 들어 있다 — 따로 더하면 입력을 두 번 세게 된다.
  it('캐시에서 읽힌 몫은 싼 단가로 친다', () => {
    const cost = estimateCost({ promptTokens: 1_000_000, completionTokens: 0, cachedTokens: 800_000, models: ['m1'] }, table);
    if (cost.kind !== 'exact') throw new Error('exact expected');
    expect(cost.usd).toBeCloseTo(0.2 * 5 + 0.8 * 0.5);
  });

  it('캐시 몫이 입력보다 클 수 없다', () => {
    const cost = estimateCost({ promptTokens: 1_000_000, completionTokens: 0, cachedTokens: 9_999_999, models: ['m1'] }, table);
    if (cost.kind !== 'exact') throw new Error('exact expected');
    expect(cost.usd).toBeCloseTo(0.5);
  });

  it('캐시 단가가 없는 모델은 입력 단가로 친다', () => {
    const cost = estimateCost({ promptTokens: 1_000_000, completionTokens: 0, cachedTokens: 1_000_000, models: ['m2'] }, table);
    if (cost.kind !== 'exact') throw new Error('exact expected');
    expect(cost.usd).toBeCloseTo(1);
  });

  it('같은 모델이 여러 번 들어와도 단일로 본다', () => {
    const cost = estimateCost({ promptTokens: 0, completionTokens: 0, models: ['m1', 'm1'] }, table);
    expect(cost.kind).toBe('exact');
  });

  // 값싼 모델과 비싼 모델의 토큰이 한 칸에 뭉쳐 있으면 어느 쪽 단가로도 몇 배씩 틀린다.
  it('모델이 섞이면 금액을 내지 않는다', () => {
    expect(estimateCost({ promptTokens: 1_000_000, completionTokens: 0, models: ['m1', 'm2'] }, table)).toEqual({
      kind: 'mixed',
      models: ['m1', 'm2'],
    });
  });

  it('단가를 모르는 모델은 unknown', () => {
    expect(estimateCost({ promptTokens: 1_000_000, completionTokens: 0, models: ['m9'] }, table)).toEqual({
      kind: 'unknown',
      model: 'm9',
    });
  });

  it('모델을 알 수 없으면 unknown', () => {
    expect(estimateCost({ promptTokens: 1, completionTokens: 1, models: [] }, table)).toEqual({ kind: 'unknown', model: null });
  });

  // 라운드 3 실측값. 단가가 바뀌면 이 값도 바뀌므로 자릿수만 지킨다.
  it('라운드 3 규모를 그럴듯한 자릿수로 낸다', () => {
    const cost = estimateCost(
      { promptTokens: 17_746_667, completionTokens: 2_837_076, models: ['anthropic/claude-opus-5'] },
      DEFAULT_PRICE_TABLE,
    );
    if (cost.kind !== 'exact') throw new Error('exact expected');
    expect(Math.round(cost.usd)).toBe(160);
    expect(cost.krw).toBeGreaterThan(200_000);
    expect(cost.krw).toBeLessThan(300_000);
  });
});

describe('costPerCharacter', () => {
  it('자수로 나눈다', () => {
    expect(costPerCharacter(3000, 1500)).toBe(2);
  });

  it('자수가 0이면 null', () => {
    expect(costPerCharacter(3000, 0)).toBeNull();
  });
});

describe('formatKrw', () => {
  it('만원 미만은 원 단위', () => {
    expect(formatKrw(9999)).toBe('9,999원');
  });

  it('만원 이상은 만원 단위', () => {
    expect(formatKrw(253_000)).toBe('25.3만원');
  });
});

describe('parsePriceTable', () => {
  it('성한 항목만 받아들인다', () => {
    const parsed = parsePriceTable({ models: { a: { input: 1, output: 2 }, b: { input: 'x' } }, usdKrw: 1300 });
    expect(parsed).toEqual({ models: { a: { input: 1, output: 2 } }, usdKrw: 1300 });
  });

  it('환율이 없거나 이상하면 기본 환율을 쓴다', () => {
    expect(parsePriceTable({ models: { a: { input: 1, output: 2 } } })?.usdKrw).toBe(DEFAULT_PRICE_TABLE.usdKrw);
    expect(parsePriceTable({ models: { a: { input: 1, output: 2 } }, usdKrw: -1 })?.usdKrw).toBe(DEFAULT_PRICE_TABLE.usdKrw);
  });

  it('쓸 만한 항목이 하나도 없으면 null', () => {
    expect(parsePriceTable({ models: {} })).toBeNull();
    expect(parsePriceTable(null)).toBeNull();
    expect(parsePriceTable('nope')).toBeNull();
  });
});

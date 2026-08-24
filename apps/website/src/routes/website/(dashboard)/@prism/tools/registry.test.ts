import { TOOL_META } from '@typie/prism';
import { describe, expect, it } from 'vitest';
import { actionOutcome } from './action-cards.ts';
import { actionCards, toolCallLabels, toolCards } from './index.ts';
import PrismActionCard from './PrismActionCard.svelte';

describe('registry ↔ TOOL_META 대조', () => {
  it('destructive 도구는 전부 정적 resolver가 user다', () => {
    let matched = 0;

    for (const [tool, meta] of Object.entries(TOOL_META)) {
      if (meta.tier !== 'destructive') continue;
      matched += 1;
      expect(meta.resolver, tool).toBe('user');
    }

    expect(matched).toBeGreaterThan(0);
  });

  it('destructive user 도구는 전부 카드가 있다', () => {
    let matched = 0;

    for (const [tool, meta] of Object.entries(TOOL_META)) {
      if (meta.tier !== 'destructive' || meta.resolver !== 'user') continue;
      matched += 1;
      expect(toolCards[tool], tool).toBeDefined();
    }

    expect(matched).toBeGreaterThan(0);
  });

  it('정책에 따라 서버 해소될 수 있는 도구는 전부 트랜스크립트 레이블이 있다', () => {
    let matched = 0;

    for (const [tool, meta] of Object.entries(TOOL_META)) {
      if (meta.resolver !== 'server' && meta.tier === undefined) continue;
      matched += 1;
      expect(toolCallLabels[tool], tool).toBeDefined();
    }

    expect(matched).toBeGreaterThan(0);
  });

  it('확인 카드 셸로 등재된 도구는 전부 본문이 있고, 본문이 있는 도구는 전부 셸로 등재돼 있다', () => {
    let matched = 0;

    for (const [tool, card] of Object.entries(toolCards)) {
      if (card !== PrismActionCard) continue;
      matched += 1;
      expect(actionCards[tool]?.body, tool).toBeDefined();
    }

    expect(matched).toBeGreaterThan(0);

    for (const tool of Object.keys(actionCards)) {
      expect(toolCards[tool], tool).toBe(PrismActionCard);
    }
  });
});

describe('확인 카드 꼬리말 판정', () => {
  it('해소 결과 봉투를 네 갈래로 가른다', () => {
    expect(actionOutcome({ status: 'resolved', result: { ok: true, count: 2 } })).toBe('done');
    expect(actionOutcome({ status: 'resolved', result: { ok: false, code: 'declined', message: '하지 않기로 했어요' } })).toBe('declined');
    expect(actionOutcome({ status: 'resolved', result: { ok: false, code: 'denied', message: '정책이 막았어요' } })).toBe('failed');
    expect(actionOutcome({ status: 'resolved', result: { ok: false, code: 'error', message: '실패했어요' } })).toBe('failed');
    expect(actionOutcome({ status: 'resolved', result: undefined })).toBe('done');
    expect(actionOutcome({ status: 'pending', result: undefined })).toBe('closed');
    expect(actionOutcome({ status: 'closed', result: undefined })).toBe('closed');
  });
});

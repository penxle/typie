import { describe, expect, it } from 'vitest';
import { pipelineUsage } from './pricing.ts';
import type { Usage } from '../../../core/contracts.ts';
import type { CallUsageRow } from './pricing.ts';

const row = (phase: string, partial: Partial<Usage>, durationMs = 0): CallUsageRow => ({
  phase,
  usage: { calls: 0, promptTokens: 0, completionTokens: 0, cachedTokens: 0, cacheWriteTokens: 0, ...partial },
  durationMs,
});

describe('pipelineUsage', () => {
  it('행이 없으면 빈 회계다', () => {
    expect(pipelineUsage([])).toEqual([]);
  });

  it('같은 단계의 호출들을 사용량·시간 모두 합친다', () => {
    const rows = [
      row('research', { calls: 1, promptTokens: 10, completionTokens: 5, cachedTokens: 2 }, 1000),
      row('research', { calls: 1, promptTokens: 20, completionTokens: 5, cacheWriteTokens: 3 }, 500),
      row('compose', { calls: 1, promptTokens: 7, completionTokens: 1 }, 200),
    ];
    expect(pipelineUsage(rows)).toEqual([
      { phase: 'research', calls: 2, promptTokens: 30, completionTokens: 10, cachedTokens: 2, cacheWriteTokens: 3, durationMs: 1500 },
      { phase: 'compose', calls: 1, promptTokens: 7, completionTokens: 1, cachedTokens: 0, cacheWriteTokens: 0, durationMs: 200 },
    ]);
  });

  it('사용량이 0이어도 시간이 있으면 단계를 유지한다 — 도구 실행 행이 시간을 나른다', () => {
    const rows = [row('research', { calls: 1, promptTokens: 10 }, 1000), row('research', {}, 300), row('tools-only', {}, 40)];
    expect(pipelineUsage(rows)).toEqual([
      { phase: 'research', calls: 1, promptTokens: 10, completionTokens: 0, cachedTokens: 0, cacheWriteTokens: 0, durationMs: 1300 },
      { phase: 'tools-only', calls: 0, promptTokens: 0, completionTokens: 0, cachedTokens: 0, cacheWriteTokens: 0, durationMs: 40 },
    ]);
  });

  it('사용량도 시간도 없는 단계는 내보내지 않는다', () => {
    const rows = [row('research', { calls: 1, promptTokens: 10 }, 1000), row('empty', {})];
    expect(pipelineUsage(rows).map((r) => r.phase)).toEqual(['research']);
  });
});

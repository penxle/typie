import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchCatalog } from '$lib/server/prism.ts';
import { startFeedbackSession } from '$lib/server/reviews.ts';
import { actions } from './+page.server.ts';
import type { AppCatalog } from '$lib/feedback/tiers.ts';

// 시작 액션의 관심사는 폼 → 티어 판정 → 시작 인자다. 반입·D1은 이 경계 밖이라 대역으로 세운다.
vi.mock('$lib/server/reviews.ts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/server/reviews.ts')>()),
  startFeedbackSession: vi.fn(),
}));

vi.mock('$lib/server/db/index.ts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/server/db/index.ts')>()),
  createDb: () => ({}),
}));

// 카탈로그도 경계 밖(prism API)이라 대역이다 — 검증·조립 재료로만 관통한다.
vi.mock('$lib/server/prism.ts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/server/prism.ts')>()),
  fetchCatalog: vi.fn(),
}));

const CATALOG = {
  models: {
    'claude-opus-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
    'claude-sonnet-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
    'gpt-5.6-luna': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
    'gemini-3.6-flash': { provider: 'gemini', efforts: ['minimal', 'low', 'medium', 'high'] },
  },
  agents: {
    'rephrase-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'critique-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
  },
  workflows: {
    high: { agents: ['description-high', 'judgment-high'] },
    medium: { agents: ['research-medium', 'rephrase-medium'] },
    low: { agents: ['critique-low'] },
  },
} satisfies AppCatalog;

type StartEvent = Parameters<(typeof actions)['start']>[0];

const started = vi.mocked(startFeedbackSession);

const run = (fields: Record<string, string>, email = 'admin@x.io') => {
  const form = new FormData();
  for (const [key, value] of Object.entries(fields)) form.append(key, value);
  const event = {
    locals: { email },
    platform: { env: { ADMIN_EMAILS: 'admin@x.io', DB: {} } },
    request: new Request('https://eval.test/', { method: 'POST', body: form }),
  } as unknown as StartEvent;
  // 성공 경로는 redirect(throw)로 끝난다 — 던진 값을 결과로 본다.
  return Promise.resolve(actions.start(event)).catch((err: unknown) => err);
};

const catalogued = vi.mocked(fetchCatalog);

beforeEach(() => {
  started.mockReset();
  started.mockResolvedValue({ sessionId: 's1' });
  catalogued.mockReset();
  catalogued.mockResolvedValue(CATALOG);
});

describe('start 액션의 티어 관통', () => {
  it('티어 미제출은 high로 시작한다', async () => {
    const outcome = await run({ documentId: 'D0TEST01' }, 't@x.io');

    expect(started).toHaveBeenCalledTimes(1);
    expect(started.mock.calls[0][2]).toEqual({ refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG, tier: 'high', overrides: {} });
    expect(outcome).toMatchObject({ status: 303, location: '/sessions/s1' });
  });

  it('빈 티어 값도 high로 굳는다', async () => {
    await run({ documentId: 'D0TEST01', tier: '' }, 't@x.io');

    expect(started.mock.calls[0][2]).toMatchObject({ tier: 'high' });
  });

  it('제출한 티어가 그대로 시작 인자에 실린다', async () => {
    await run({ documentId: 'D0TEST01', tier: 'low' });

    expect(started.mock.calls[0][2]).toMatchObject({ tier: 'low' });
  });

  it('hidden 오버라이드 필드를 접미사 붙은 에이전트 이름으로 걷는다', async () => {
    await run({
      documentId: 'D0TEST01',
      tier: 'medium',
      'tier.rephrase-medium.model': 'gpt-5.6-luna',
      'tier.rephrase-medium.effort': 'low',
    });

    expect(started.mock.calls[0][2]).toMatchObject({
      tier: 'medium',
      overrides: { 'rephrase-medium': { provider: 'openai', model: 'gpt-5.6-luna', effort: 'low' } },
    });
  });

  it('미지 에이전트 키는 무음 탈락이 아니라 400이다', async () => {
    const outcome = await run({
      documentId: 'D0TEST01',
      tier: 'high',
      'tier.Research_High.model': 'claude-opus-5',
      'tier.Research_High.effort': 'xhigh',
    });

    expect(outcome).toMatchObject({ status: 400 });
    expect(started).not.toHaveBeenCalled();
  });

  it('운영자가 아닌 티어 선택은 400으로 막고 아무것도 시작하지 않는다', async () => {
    const outcome = await run({ documentId: 'D0TEST01', tier: 'low' }, 't@x.io');

    expect(outcome).toMatchObject({ status: 400 });
    expect(started).not.toHaveBeenCalled();
  });

  it('알 수 없는 티어는 운영자 제출이라도 400이다', async () => {
    const outcome = await run({ documentId: 'D0TEST01', tier: 'extreme' });

    expect(outcome).toMatchObject({ status: 400 });
    expect(started).not.toHaveBeenCalled();
  });

  it('카탈로그를 못 걷으면 502로 명시 실패하고 아무것도 시작하지 않는다 — 폴백 없음', async () => {
    catalogued.mockRejectedValue(new Error('down'));

    const outcome = await run({ documentId: 'D0TEST01' }, 't@x.io');

    expect(outcome).toMatchObject({ status: 502 });
    expect(started).not.toHaveBeenCalled();
  });
});

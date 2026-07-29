import { error, fail } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { costPerCharacter, sumCosts } from '$lib/domain/pricing.ts';
import { phaseCosts, readPriceTable, runCost, totalUsage } from '$lib/server/pricing.ts';
import { cancelRun, isRunLocked, refreshRun, retryRun } from '$lib/server/run-service.ts';
import { loadRunView } from '$lib/server/run-view.ts';
import { createDb, Ledgers, PhaseUsage, PromptSets, Runs } from '../../../../../core/db.ts';
import { generationById } from '../../../../../core/registry.ts';
import type { ToolRecord } from '../../../../../core/contracts.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  await refreshRun(db, platform.env, params.id);

  const [run] = await db.select().from(Runs).where(eq(Runs.id, params.id));
  if (!run) error(404, 'run not found');

  const view = await loadRunView(db, params.id);
  if (!view) error(500, 'run view missing');

  const manifest = generationById(view.generationId ?? '');
  const usage = await db.select().from(PhaseUsage).where(eq(PhaseUsage.runId, params.id));

  const [set] = run.promptSetId
    ? await db.select({ content: PromptSets.content }).from(PromptSets).where(eq(PromptSets.id, run.promptSetId))
    : [];
  const table = await readPriceTable(db);
  const content = set?.content ?? null;
  const costs = new Map(phaseCosts(usage, content, table).map((row) => [row.phase, row]));
  // 실행 단위로는 모델이 섞여 금액이 안 나와도 단계별 합으로는 나온다.
  const stageTotal = usage.length > 0 ? sumCosts([...costs.values()].map((c) => c.cost)) : null;
  const cost = runCost(usage, content, table);
  const totals = totalUsage(usage);
  // 원장은 턴마다 갱신되므로 진행 중인 단계도 그대로 읽으면 된다.
  const ledgerRows = await db.select().from(Ledgers).where(eq(Ledgers.runId, params.id));
  // 원장은 턴마다 갱신되므로 진행 중인 단계도 그대로 읽으면 된다.
  // 순서는 기록 시각으로 낸다 — 매니페스트에 없는 키가 섞여도 실제 실행 순서대로 놓인다.
  const ledgers = ledgerRows
    .filter((row) => row.key.startsWith('ledger/'))
    .map((row) => {
      const stage = row.key.slice('ledger/'.length);
      const value = row.value as { tools?: unknown; events?: unknown };
      return {
        stage,
        at: row.createdAt.getTime(),
        label: manifest?.phases.find((p) => p.key === stage)?.label ?? stage,
        tools: Array.isArray(value.tools) ? (value.tools as ToolRecord[]) : [],
        events: Array.isArray(value.events) ? (value.events as { turn?: number; kind: string; detail: string }[]) : [],
      };
    })
    .toSorted((a, b) => a.at - b.at);

  return {
    run: { ...run, createdAt: run.createdAt.toISOString(), finishedAt: run.finishedAt?.toISOString() ?? null },
    view,
    // 진행 표시와 비용표가 같은 축이다 — 매니페스트의 phases 하나로 둘 다 그린다.
    phases:
      manifest?.phases.map((p) => ({
        ...p,
        usage: usage.find((u) => u.phase === p.key) ?? null,
        model: costs.get(p.key)?.model ?? null,
        cost: costs.get(p.key)?.cost ?? null,
      })) ?? [],
    orphanUsage: usage
      .filter((u) => !manifest?.phases.some((p) => p.key === u.phase))
      .map((u) => ({ ...u, model: costs.get(u.phase)?.model ?? null, cost: costs.get(u.phase)?.cost ?? null })),
    cost,
    stageTotal,
    krwPerCharacter:
      stageTotal?.complete && stageTotal.krw > 0
        ? costPerCharacter(stageTotal.krw, view.document.characterCount)
        : cost.kind === 'exact'
          ? costPerCharacter(cost.krw, view.document.characterCount)
          : null,
    characters: view.document.characterCount,
    models: [...new Set([...costs.values()].map((c) => c.model).filter((m): m is string => m !== null))],
    tokens: totals.promptTokens + totals.completionTokens,
    ledgers,
    // 앵커가 본문에서 실제로 잡혔는가 — 산출물이 원고를 가리키는지 보는 유일한 기계 지표다.
    metrics: {
      anchorMatchRate: (() => {
        const anchors = view.items.flatMap((i) => i.anchors);
        return anchors.length === 0 ? NaN : anchors.filter((a) => a.matchStart !== null).length / anchors.length;
      })(),
      findings: view.items.filter((i) => i.kind === 'finding').length,
      reviewItems: view.items.filter((i) => i.kind !== 'finding').length,
    },
    locked: await isRunLocked(db, params.id),
  };
};

export const actions: Actions = {
  retry: async ({ params, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const result = await retryRun(createDb(platform.env.DB), platform.env, params.id);
    return 'error' in result ? fail(400, { message: result.error }) : { ok: true };
  },
  cancel: async ({ params, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const result = await cancelRun(createDb(platform.env.DB), platform.env, params.id);
    return 'error' in result ? fail(400, { message: result.error }) : { ok: true };
  },
};

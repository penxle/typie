import { error, fail } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { costPerCharacter, sumCosts } from '$lib/domain/pricing.ts';
import { phaseCosts, pipelineUsage, readPriceTable, runCost, totalUsage } from '$lib/server/pricing.ts';
import { cancelRun, isRunLocked, refreshRun, retryRun } from '$lib/server/run-service.ts';
import { loadRunView } from '$lib/server/run-view.ts';
import { CallUsage, createDb, Ledgers, PromptSets, Runs } from '../../../../../core/db.ts';
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
  // 회계는 호출별 원장(call_usage) 하나에서만 나온다 — phase_usage는 재시도 비용까지 누적되는
  // 진단 기록이라 화면에 쓰지 않는다.
  const calls = await db
    .select({ phase: CallUsage.phase, usage: CallUsage.usage, durationMs: CallUsage.durationMs })
    .from(CallUsage)
    .where(eq(CallUsage.runId, params.id));
  const usage = pipelineUsage(calls);

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
      const value = row.value as { tools?: unknown; events?: unknown; turns?: unknown; scratchFiles?: unknown };
      return {
        stage,
        at: row.createdAt.getTime(),
        label: manifest?.phases.find((p) => p.key === stage)?.label ?? stage,
        tools: Array.isArray(value.tools) ? (value.tools as ToolRecord[]) : [],
        events: Array.isArray(value.events) ? (value.events as { turn?: number; kind: string; detail: string }[]) : [],
        // 턴 기록은 이 기능 도입 이후의 실행에만 있다 — 없으면 빈 배열로 두고 화면이 접는다.
        turns: Array.isArray(value.turns)
          ? (value.turns as { stage: string; turn: number; thinking?: string; text: string; submissions: string[] }[])
          : [],
        // 스테이지 완료 시점의 scratch/ 스냅샷 — 파일시스템 전환 이후의 실행에만 있다.
        scratchFiles: Array.isArray(value.scratchFiles) ? (value.scratchFiles as { path: string; content: string }[]) : [],
      };
    })
    .toSorted((a, b) => a.at - b.at);

  // 완료된 실행의 소요는 파이프라인 1회분(원장 합), 진행 중은 벽시계 경과 — 목록과 같은 규칙.
  // 합 0은 미기록(시간 기록이 없던 시절의 백필)이다.
  const started = run.startedAt ?? run.createdAt;
  const pipelineSeconds = usage.reduce((sum, p) => sum + p.durationMs, 0) / 1000;
  const durationSeconds = run.status === 'running' ? (Date.now() - started.getTime()) / 1000 : pipelineSeconds > 0 ? pipelineSeconds : null;

  return {
    run: {
      ...run,
      createdAt: run.createdAt.toISOString(),
      startedAt: run.startedAt?.toISOString() ?? null,
      finishedAt: run.finishedAt?.toISOString() ?? null,
    },
    durationSeconds,
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

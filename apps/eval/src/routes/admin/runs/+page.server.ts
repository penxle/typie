import { error, fail } from '@sveltejs/kit';
import { desc, eq } from 'drizzle-orm';
import { costPerCharacter, sumCosts } from '$lib/domain/pricing.ts';
import { phaseCosts, pipelineUsage, readPriceTable, runCost, totalUsage } from '$lib/server/pricing.ts';
import { cancelRun, refreshRun, retryRun } from '$lib/server/run-service.ts';
import { CallUsage, createDb, Documents, PromptSets, Runs } from '../../../../core/db.ts';
import { generationById } from '../../../../core/registry.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const rows = await db
    .select({
      id: Runs.id,
      status: Runs.status,
      phase: Runs.phase,
      error: Runs.error,
      createdAt: Runs.createdAt,
      startedAt: Runs.startedAt,
      refId: Documents.refId,
      characterCount: Documents.characterCount,
      promptSetLabel: PromptSets.label,
      promptSetContent: PromptSets.content,
      generationId: PromptSets.generationId,
    })
    .from(Runs)
    .leftJoin(Documents, eq(Documents.id, Runs.documentId))
    .leftJoin(PromptSets, eq(PromptSets.id, Runs.promptSetId))
    .orderBy(desc(Runs.createdAt));

  // 진행 중인 것만 인스턴스 상태를 물어 반영한다 — 워커가 죽으면 DB에 실패를 못 쓴다.
  // 이 대사는 이번 응답에 기여하지 않는다(화면은 위 SELECT를 그대로 내보내고 교정은 다음
  // 폴링에서 보인다) — 응답을 잡아두지 않도록 반환 뒤 백그라운드로 미룬다.
  const active = rows.filter((r) => r.status === 'running' || r.status === 'pending');
  const env = platform.env;
  platform.context?.waitUntil(
    (async () => {
      for (let i = 0; i < active.length; i += 10) {
        await Promise.all(active.slice(i, i + 10).map((row) => refreshRun(db, env, row.id)));
      }
    })().catch((err) => console.warn('run 상태 대사 실패', err)),
  );

  // 비용은 단계별 사용량을 실행 단위로 합쳐 낸다 — 단계마다 모델이 다르면 '혼합'으로 떨어진다.
  // 회계는 호출별 원장(call_usage) 하나에서만 나온다 — phase_usage는 재시도 비용까지 누적되는
  // 진단 기록이라 화면에 쓰지 않는다.
  const calls = await db
    .select({ runId: CallUsage.runId, phase: CallUsage.phase, usage: CallUsage.usage, durationMs: CallUsage.durationMs })
    .from(CallUsage);
  const table = await readPriceTable(db);
  const callsByRun = new Map<string, typeof calls>();
  for (const row of calls) callsByRun.set(row.runId, [...(callsByRun.get(row.runId) ?? []), row]);

  return {
    runs: rows.map((r) => {
      const mine = pipelineUsage(callsByRun.get(r.id) ?? []);
      const totals = totalUsage(mine);
      const { promptSetContent, startedAt, ...rest } = r;
      const cost = runCost(mine, promptSetContent, table);
      const stageTotal = mine.length > 0 ? sumCosts(phaseCosts(mine, promptSetContent, table).map((c) => c.cost)) : null;
      const krw = stageTotal?.complete && stageTotal.krw > 0 ? stageTotal.krw : cost.kind === 'exact' ? cost.krw : null;
      const started = startedAt ?? r.createdAt;
      // 완료된 실행의 소요는 파이프라인 1회분(원장 합)이다 — 비용과 같은 축이라 캐시로 이어
      // 돌린 재실행에서도 원래 걸린 시간이 보인다. 벽시계는 진행 중 표시에만 쓴다 — 라이브
      // 표시가 묻는 것은 "얼마나 지났나(멈췄나)"이고, 원장 합은 호출이 끝나야만 자란다.
      // 합 0은 미기록(시간 기록이 없던 시절의 백필)이다.
      const pipelineSeconds = mine.reduce((sum, p) => sum + p.durationMs, 0) / 1000;
      return {
        ...rest,
        createdAt: r.createdAt.toISOString(),
        phaseLabel: generationById(r.generationId ?? '')?.phases.find((p) => p.key === r.phase)?.label ?? r.phase,
        cost,
        // 실행 단위로 '혼합'이 되어도 단계별 합으로는 정확한 금액이 나온다.
        stageTotal,
        tokens: totals.promptTokens + totals.completionTokens,
        krwPerCharacter: krw !== null && r.characterCount ? costPerCharacter(krw, r.characterCount) : null,
        durationSeconds: r.status === 'running' ? (Date.now() - started.getTime()) / 1000 : pipelineSeconds > 0 ? pipelineSeconds : null,
      };
    }),
  };
};

export const actions: Actions = {
  retry: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    const id = String(form.get('id') ?? '');
    const result = await retryRun(createDb(platform.env.DB), platform.env, id);
    return 'error' in result ? fail(400, { message: result.error }) : { ok: true };
  },
  cancel: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    const id = String(form.get('id') ?? '');
    const result = await cancelRun(createDb(platform.env.DB), platform.env, id);
    return 'error' in result ? fail(400, { message: result.error }) : { ok: true };
  },
};

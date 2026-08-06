import { and, eq, sql } from 'drizzle-orm';
import { emptyUsage } from '../../core/contracts.ts';
import { CallCache, CallUsage, Ledgers, PhaseUsage, Runs } from '../../core/db.ts';
import type { WorkflowStep } from 'cloudflare:workers';
import type { PhasePrompt, Usage } from '../../core/contracts.ts';
import type { Db } from '../../core/db.ts';
import type { RunContext, RunEnv } from '../../core/worker/run-contracts.ts';

export const accumulate = (a: Usage, b: Usage): Usage => ({
  calls: Math.round(a.calls + b.calls),
  promptTokens: Math.round(a.promptTokens + b.promptTokens),
  completionTokens: Math.round(a.completionTokens + b.completionTokens),
  cachedTokens: Math.round(a.cachedTokens + b.cachedTokens),
  cacheWriteTokens: Math.round(a.cacheWriteTokens + b.cacheWriteTokens),
});

const addPhaseUsage = async (db: Db, runId: string, phase: string, usage: Usage): Promise<void> => {
  if (usage.promptTokens === 0 && usage.completionTokens === 0) return;
  const row = accumulate(emptyUsage(), usage);
  await db
    .insert(PhaseUsage)
    .values({ runId, phase, ...row })
    .onConflictDoUpdate({
      target: [PhaseUsage.runId, PhaseUsage.phase],
      set: {
        calls: sql`${PhaseUsage.calls} + ${row.calls}`,
        promptTokens: sql`${PhaseUsage.promptTokens} + ${row.promptTokens}`,
        completionTokens: sql`${PhaseUsage.completionTokens} + ${row.completionTokens}`,
        cachedTokens: sql`${PhaseUsage.cachedTokens} + ${row.cachedTokens}`,
        cacheWriteTokens: sql`${PhaseUsage.cacheWriteTokens} + ${row.cacheWriteTokens}`,
      },
    });
};

export const createRunContext = (deps: {
  db: Db;
  step: WorkflowStep;
  env: RunEnv;
  runId: string;
  document: { id: string; refId: string; content: string };
  prompts: Record<string, PhasePrompt>;
}): RunContext => {
  let current = 'unknown';

  return {
    step: deps.step,
    env: deps.env,
    document: deps.document,
    prompts: deps.prompts,

    phase: async (key) => {
      current = key;
      await deps.db.update(Runs).set({ phase: key }).where(eq(Runs.id, deps.runId));
    },

    // 캐시와 비용 회계를 한 호출로 묶는다. 둘이 갈려 있으면 짝이 깨져도 아무 신호가 없어
    // 어느 단계에서 회계를 빠뜨렸는지 알 방법이 없다.
    cached: async (key, fn) => {
      const [hit] = await deps.db
        .select({ value: CallCache.value })
        .from(CallCache)
        .where(and(eq(CallCache.runId, deps.runId), eq(CallCache.key, key)))
        .limit(1);
      // 적중 시 비용을 더하지 않는다 — 저장분을 다시 더하면 리플레이마다 이중 계상된다.
      if (hit) return { value: (hit.value as { value: unknown }).value as never, cached: true };

      const usage = emptyUsage();
      const startedAt = Date.now();
      const value = await fn(usage);
      const row = accumulate(emptyUsage(), usage);
      const durationMs = Math.max(0, Math.round(Date.now() - startedAt));
      // 원장을 캐시보다 먼저 쓴다. 캐시가 먼저 들어가고 그 사이에 실패하면 다음 시도는 적중으로
      // 끝나 이 호출의 회계가 영영 빠진다 — 원장이 먼저면 실패해도 다음 시도가 재실행해 덮어쓴다.
      await deps.db
        .insert(CallUsage)
        .values({ runId: deps.runId, key, phase: current, usage: row, durationMs })
        .onConflictDoUpdate({ target: [CallUsage.runId, CallUsage.key], set: { phase: current, usage: row, durationMs } });
      // 값을 한 겹 감싼다. 러너가 null이나 배열을 반환해도 적중 판별이 무너지지 않게.
      await deps.db.insert(CallCache).values({ runId: deps.runId, key, value: { value } }).onConflictDoNothing();
      await addPhaseUsage(deps.db, deps.runId, current, usage);
      return { value, cached: false };
    },

    // 단계 도중에도 갱신된다 — 같은 키로 다시 쓰면 최신본이 이긴다.
    ledger: async (key, value) => {
      await deps.db
        .insert(Ledgers)
        .values({ runId: deps.runId, key, value })
        .onConflictDoUpdate({ target: [Ledgers.runId, Ledgers.key], set: { value } });
    },
  };
};

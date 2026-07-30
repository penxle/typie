import { and, eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { Documents, inChunks, Judgments, PromptSets, Runs, Tasks } from '../../../core/db.ts';
import { generationById } from '../../../core/registry.ts';
import type { Db } from '../../../core/db.ts';

type Env = App.Platform['env'];

export const spawnPlan = (generation: { status: string }, documentIds: string[]): { ok: true } | { error: string } => {
  if (generation.status !== 'active') return { error: '동결된 세대는 실행할 수 없습니다' };
  if (documentIds.length === 0) return { error: '실행할 문서가 없습니다' };
  return { ok: true };
};

// 항목 id가 바뀌면 judgment_items가 존재하지 않는 항목을 가리킨다. 태스크만 있고 아직 아무도
// 받지 않았으면 잠기지 않는다.
export const isRunLocked = async (db: Db, runId: string): Promise<boolean> => {
  const [row] = await db
    .select({ id: Judgments.id })
    .from(Judgments)
    .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
    .where(eq(Tasks.runId, runId))
    .limit(1);
  return !!row;
};

export const spawnRuns = async (
  db: Db,
  env: Env,
  input: { promptSetId: string; documentIds: string[] },
): Promise<{ spawned: string[]; failed: { documentId: string; error: string }[] } | { error: string }> => {
  const [set] = await db.select().from(PromptSets).where(eq(PromptSets.id, input.promptSetId)).limit(1);
  if (!set) return { error: 'prompt set not found' };
  const manifest = generationById(set.generationId);
  if (!manifest) return { error: `generation module missing: ${set.generationId}` };

  const docs =
    input.documentIds.length > 0
      ? await inChunks(input.documentIds, (chunk) => db.select({ id: Documents.id }).from(Documents).where(inArray(Documents.id, chunk)))
      : [];

  const gate = spawnPlan(
    manifest,
    docs.map((d) => d.id),
  );
  if ('error' in gate) return gate;
  // 지정한 문서가 없으면 조용히 적게 도는 대신 실패시킨다 — 부분집합 실행의 결과를 전체
  // 실행과 헷갈리게 두지 않는다.
  if (docs.length !== input.documentIds.length) {
    return { error: `문서를 찾을 수 없습니다: ${input.documentIds.filter((id) => docs.every((d) => d.id !== id)).join(', ')}` };
  }

  // 연타·동시 요청 방어. 같은 세트로 이미 돌고 있는 문서를 다시 걸면 같은 결과에 두 번
  // 과금된다 — 클라이언트 버튼 상태는 요청이 날아가는 동안의 연타를 못 막으므로 서버가
  // 최종 관문이다. 완료·실패한 실행의 재실행은 retryRun의 몫이라 여기서 막지 않는다.
  const active = await inChunks(input.documentIds, (chunk) =>
    db
      .select({ documentId: Runs.documentId })
      .from(Runs)
      .where(and(eq(Runs.promptSetId, input.promptSetId), inArray(Runs.documentId, chunk), inArray(Runs.status, ['pending', 'running']))),
  );
  if (active.length > 0) {
    return { error: `이 세트로 이미 실행 중인 문서가 ${active.length}건 있습니다 — 완료를 기다리거나 취소 후 다시 시도하세요` };
  }

  const spawned: string[] = [];
  const failed: { documentId: string; error: string }[] = [];
  for (const doc of docs) {
    const runId = nanoid();
    await db.insert(Runs).values({ id: runId, documentId: doc.id, promptSetId: input.promptSetId, status: 'pending' });
    try {
      const instance = await env.RUN.create({ params: { runId } });
      await db.update(Runs).set({ instanceId: instance.id }).where(eq(Runs.id, runId));
      spawned.push(runId);
    } catch (err) {
      const message = String(err).slice(0, 1000);
      await db.update(Runs).set({ status: 'failed', error: message }).where(eq(Runs.id, runId));
      failed.push({ documentId: doc.id, error: message });
    }
  }
  return { spawned, failed };
};

// 같은 runs 행을 다시 건다. call_cache가 남아 있어 끝난 호출은 재사용되고 실패 지점부터 잇는다.
export const retryRun = async (db: Db, env: Env, runId: string): Promise<{ ok: true } | { error: string }> => {
  const [run] = await db.select().from(Runs).where(eq(Runs.id, runId)).limit(1);
  if (!run) return { error: 'run not found' };
  if (await isRunLocked(db, runId)) return { error: '판정이 걸린 실행은 다시 돌릴 수 없습니다' };

  await db
    .update(Runs)
    .set({ status: 'pending', phase: null, error: null, instanceId: null, startedAt: null, finishedAt: null })
    .where(eq(Runs.id, runId));
  try {
    const instance = await env.RUN.create({ params: { runId } });
    await db.update(Runs).set({ instanceId: instance.id }).where(eq(Runs.id, runId));
    return { ok: true };
  } catch (err) {
    const message = String(err).slice(0, 1000);
    await db.update(Runs).set({ status: 'failed', error: message }).where(eq(Runs.id, runId));
    return { error: message };
  }
};

export const cancelRun = async (db: Db, env: Env, runId: string): Promise<{ ok: true } | { error: string }> => {
  const [run] = await db.select().from(Runs).where(eq(Runs.id, runId)).limit(1);
  if (!run) return { error: 'run not found' };
  // 이미 끝난 실행을 취소로 덮으면 산출물이 있는데도 취소로 보인다.
  if (run.status !== 'running' && run.status !== 'pending') return { error: '진행 중인 실행만 취소할 수 있습니다' };
  if (run.instanceId) {
    try {
      const instance = await env.RUN.get(run.instanceId);
      const status = await instance.status();
      if (status.status === 'running' || status.status === 'queued') await instance.terminate();
    } catch {
      // 인스턴스에 닿지 못해도 아래에서 cancelled로 표시한다 — 호출 캐시와 원장은 남는다
    }
  }
  await db.update(Runs).set({ status: 'cancelled', phase: null, finishedAt: new Date() }).where(eq(Runs.id, runId));
  return { ok: true };
};

// 워커가 죽으면 DB에 failed를 못 쓴다. 폴링할 때 인스턴스 상태를 물어 반영한다.
export const refreshRun = async (db: Db, env: Env, runId: string): Promise<void> => {
  const [run] = await db.select().from(Runs).where(eq(Runs.id, runId)).limit(1);
  if (!run?.instanceId || (run.status !== 'running' && run.status !== 'pending')) return;
  try {
    const instance = await env.RUN.get(run.instanceId);
    const status = await instance.status();
    if (status.status === 'errored') {
      await db
        .update(Runs)
        .set({ status: 'failed', error: (status.error?.message ?? 'workflow errored').slice(0, 1000), finishedAt: new Date() })
        .where(eq(Runs.id, runId));
    } else if (status.status === 'terminated') {
      await db.update(Runs).set({ status: 'cancelled', finishedAt: new Date() }).where(eq(Runs.id, runId));
    }
  } catch {
    // 워커에 닿지 못하면 상태를 그대로 둔다 (로컬 개발 등)
  }
};

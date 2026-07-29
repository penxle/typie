import { desc, eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { Samplings } from '../../../core/db.ts';
import type { Db } from '../../../core/db.ts';

type Env = App.Platform['env'];

// 표집은 실행과 별개의 축이다 — 문서를 만들어 넣는 일이라 runs에 섞지 않는다.
export const spawnSampling = async (db: Db, env: Env, size: number): Promise<{ id: string } | { error: string }> => {
  if (!Number.isSafeInteger(size) || size < 1) return { error: '크기는 1 이상의 정수여야 합니다' };

  // 연타·동시 요청 방어. 표집이 겹치면 같은 후보를 중복 심사하고 같은 원고가 문서로 거듭 들어온다
  // — 클라이언트 버튼 상태는 요청이 날아가는 동안의 연타를 못 막으므로 서버가 최종 관문이다.
  const [active] = await db
    .select({ id: Samplings.id })
    .from(Samplings)
    .where(inArray(Samplings.status, ['pending', 'running']))
    .limit(1);
  if (active) return { error: '이미 진행 중인 표집이 있습니다' };

  const id = nanoid();
  await db.insert(Samplings).values({ id, status: 'pending', size });
  try {
    const instance = await env.SAMPLING.create({ params: { runId: id, size } });
    await db.update(Samplings).set({ instanceId: instance.id, status: 'running' }).where(eq(Samplings.id, id));
    return { id };
  } catch (err) {
    const message = String(err).slice(0, 1000);
    await db.update(Samplings).set({ status: 'failed', error: message }).where(eq(Samplings.id, id));
    return { error: message };
  }
};

// 워커가 죽으면 DB에 failed를 못 쓴다. 그대로 두면 'running' 잔재가 위 가드를 영구히 잠근다 —
// 폴링할 때 인스턴스 상태를 물어 반영한다.
export const refreshSampling = async (db: Db, env: Env, samplingId: string): Promise<void> => {
  const [row] = await db.select().from(Samplings).where(eq(Samplings.id, samplingId)).limit(1);
  if (!row?.instanceId || (row.status !== 'running' && row.status !== 'pending')) return;
  try {
    const instance = await env.SAMPLING.get(row.instanceId);
    const status = await instance.status();
    if (status.status === 'errored') {
      await db
        .update(Samplings)
        .set({ status: 'failed', phase: null, error: (status.error?.message ?? 'workflow errored').slice(0, 1000), finishedAt: new Date() })
        .where(eq(Samplings.id, samplingId));
    } else if (status.status === 'terminated') {
      await db.update(Samplings).set({ status: 'cancelled', phase: null, finishedAt: new Date() }).where(eq(Samplings.id, samplingId));
    }
  } catch {
    // 워커에 닿지 못하면 상태를 그대로 둔다 (로컬 개발 등)
  }
};

export const recentSamplings = async (db: Db) => {
  const rows = await db.select().from(Samplings).orderBy(desc(Samplings.createdAt)).limit(10);
  return rows.map((r) => ({
    id: r.id,
    status: r.status,
    phase: r.phase,
    size: r.size,
    error: r.error,
    createdAt: r.createdAt.toISOString(),
    finishedAt: r.finishedAt?.toISOString() ?? null,
  }));
};

import { error } from '@sveltejs/kit';
import { inArray } from 'drizzle-orm';
import { createDb, Variants } from '$lib/server/db/index.ts';
import type { RunKind, RunPhase, RunStatus } from '$lib/domain/admin-types.ts';
import type { PageServerLoad } from './$types';

type RunRow = {
  id: string;
  kind: RunKind;
  variantId: string | null;
  corpusVersion: string;
  status: RunStatus;
  phase: RunPhase | null;
  doneChunks: number;
  totalChunks: number;
  doneDocs: number;
  totalDocs: number;
  createdAt: string;
};

export const load: PageServerLoad = async ({ platform, locals, fetch }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  // U1의 홈 페이지와 동일한 이유로 admin API를 self-fetch한다: GET이 폴링 시점에 워크플로 상태를 갱신(refreshRun)하므로
  // 목록의 상태가 오래된 값을 보이지 않는다. 내부 fetch는 hooks.server.ts의 인증 검사를 다시 통과해야 하므로
  // 원 요청에서 이미 확인된 이메일을 헤더로 그대로 전달한다.
  const runsRes = await fetch('/admin/api/runs', { headers: { 'cf-access-authenticated-user-email': locals.email } });
  if (!runsRes.ok) {
    error(runsRes.status, 'failed to load runs');
  }
  const { runs } = (await runsRes.json()) as { runs: RunRow[] };

  const db = createDb(platform.env.DB);
  const variantIds = [...new Set(runs.map((r) => r.variantId).filter((id): id is string => id !== null))];
  const variants =
    variantIds.length > 0
      ? await db.select({ id: Variants.id, label: Variants.label }).from(Variants).where(inArray(Variants.id, variantIds))
      : [];
  const labelById = new Map(variants.map((v) => [v.id, v.label]));

  return {
    runs: runs.map((run) => ({ ...run, variantLabel: run.variantId ? (labelById.get(run.variantId) ?? run.variantId) : null })),
  };
};

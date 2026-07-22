import { error } from '@sveltejs/kit';
import { desc, sql } from 'drizzle-orm';
import { createDb, Documents, PromptVariants } from '$lib/server/db/index.ts';
import { loadVariantStatusIndex } from './lib/status.ts';
import type { RunKind, RunStatus } from '$lib/domain/admin-types.ts';
import type { PageServerLoad } from './$types';

type RunSummary = { id: string; kind: RunKind; variantId: string | null; status: RunStatus; createdAt: string };

export const load: PageServerLoad = async ({ platform, locals, fetch }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  // 실행 목록은 기존 admin API를 통해 조회한다 — GET이 폴링 시점에 워크플로 상태를 갱신(refreshRun)하므로
  // "진행 중 run 수"가 오래된 값을 보이지 않는다. 내부 fetch는 hooks.server.ts의 인증 검사를 다시 통과해야 하므로
  // 원 요청에서 이미 확인된 이메일을 헤더로 그대로 전달한다. draft/ran/adopted/applied 판정에는 이 목록을 쓰지 않는다 —
  // 이 라우트는 .limit(100)이라 오래된 run이 빠질 수 있어, ./lib/status.ts가 PipelineRuns를 무제한으로 별도 조회한다.
  const runsRes = await fetch('/admin/api/runs', { headers: { 'cf-access-authenticated-user-email': locals.email } });
  if (!runsRes.ok) {
    error(runsRes.status, 'failed to load runs');
  }
  const { runs } = (await runsRes.json()) as { runs: RunSummary[] };

  const runningRuns = runs.filter((r) => r.status === 'running');
  const latestRunningRun = runningRuns.at(0) ?? null;

  // content 컬럼은 이 페이지에서 쓰지 않으므로(라벨/상태만 필요) 오버페치를 피해 필요한 컬럼만 선택한다.
  const [variants, statusIndex] = await Promise.all([
    db.select({ id: PromptVariants.id, label: PromptVariants.label }).from(PromptVariants).orderBy(desc(PromptVariants.createdAt)),
    loadVariantStatusIndex(db),
  ]);

  const variantLabelById = new Map(variants.map((v) => [v.id, v.label]));

  const currentByStage = statusIndex.currentByStage.map((entry) => ({
    stage: entry.stage,
    variantLabel: entry.appliedVariantId ? (variantLabelById.get(entry.appliedVariantId) ?? entry.appliedVariantId) : null,
    appliedAt: entry.appliedAt,
  }));

  const variantSummaries = variants.map((v) => ({ id: v.id, label: v.label, status: statusIndex.deriveStatus(v.id) }));

  const corpusVersions = await db
    .select({ corpusVersion: Documents.corpusVersion, latestAt: sql<number>`max(${Documents.createdAt})` })
    .from(Documents)
    .groupBy(Documents.corpusVersion)
    .orderBy(sql`max(${Documents.createdAt}) desc`);

  const nextAction =
    variants.length === 0
      ? ({ kind: 'create-variant' } as const)
      : runs.length === 0
        ? ({ kind: 'run' } as const)
        : latestRunningRun
          ? ({ kind: 'view-run', runId: latestRunningRun.id } as const)
          : ({ kind: 'all-clear' } as const);

  return {
    runningCount: runningRuns.length,
    currentByStage,
    latestCorpusVersion: corpusVersions[0]?.corpusVersion ?? null,
    variantSummaries,
    nextAction,
  };
};

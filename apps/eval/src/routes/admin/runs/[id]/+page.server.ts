import { error } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import { anchorMatchRate, categoryComplianceRate, feedbackCountDistribution } from '$lib/domain/aggregate.ts';
import { createDb, Documents, Feedbacks, FeedbackSets, Variants } from '$lib/server/db/index.ts';
import type { RunDocStatus, RunKind, RunPhase, RunStatus } from '$lib/domain/admin-types.ts';
import type { PageServerLoad } from './$types';

type RunDetail = {
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
  promptTokens: number;
  completionTokens: number;
  error: string | null;
  createdAt: string;
  finishedAt: string | null;
};

type RunDocRow = {
  id: string;
  runId: string;
  documentId: string;
  workflowInstanceId: string | null;
  status: RunDocStatus;
  doneChunks: number;
  totalChunks: number;
  error: string | null;
};

export const load: PageServerLoad = async ({ params, platform, locals, fetch }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  // 목록과 동일하게 admin API를 self-fetch — GET이 refreshRun을 실행해 워크플로 인스턴스 상태를 최신화한다.
  const res = await fetch(`/admin/api/runs/${params.id}`, { headers: { 'cf-access-authenticated-user-email': locals.email } });
  if (!res.ok) {
    error(res.status, 'failed to load run');
  }
  const { run, docs } = (await res.json()) as { run: RunDetail; docs: RunDocRow[] };

  const db = createDb(platform.env.DB);

  let variantLabel: string | null = null;
  if (run.variantId) {
    const [variant] = await db.select({ label: Variants.label }).from(Variants).where(eq(Variants.id, run.variantId)).limit(1);
    variantLabel = variant?.label ?? run.variantId;
  }

  let summary: {
    anchorMatchRate: number;
    feedbackDistribution: { zero: number; total: number };
    categoryCompliance: number;
  } | null = null;
  let preview: {
    documentId: string;
    refId: string;
    feedbacks: { id: string; category: string | null; body: string; matchStart: number | null }[];
  }[] = [];

  // 완료된 파이프라인 실행에서만 기계 지표·프리뷰를 계산한다(브리프: "완료 시"). feedbacks/documents는 읽기 전용 select.
  if (run.status === 'succeeded' && run.kind === 'pipeline') {
    const sets = await db.select().from(FeedbackSets).where(eq(FeedbackSets.runId, run.id));
    const setIds = sets.map((s) => s.id);
    const feedbacks = setIds.length > 0 ? await db.select().from(Feedbacks).where(inArray(Feedbacks.setId, setIds)) : [];
    const docIds = [...new Set(sets.map((s) => s.documentId))];
    const documents =
      docIds.length > 0
        ? await db.select({ id: Documents.id, refId: Documents.refId }).from(Documents).where(inArray(Documents.id, docIds))
        : [];
    const refIdByDoc = new Map(documents.map((d) => [d.id, d.refId]));

    // aggregate 함수는 variantId로 그룹핑하지만 이 화면은 실행 하나=variant 하나이므로 그룹 키로만 사용한다.
    const groupKey = run.variantId ?? run.id;
    const anchorEntries = sets.map((s) => {
      const setFeedbacks = feedbacks.filter((f) => f.setId === s.id);
      return {
        variantId: groupKey,
        matchedCount: setFeedbacks.filter((f) => f.matchStart !== null).length,
        feedbackCount: setFeedbacks.length,
      };
    });
    const countEntries = sets.map((s) => ({ variantId: groupKey, feedbackCount: feedbacks.filter((f) => f.setId === s.id).length }));
    const categories = feedbacks.map((f) => f.category);

    summary = {
      anchorMatchRate: anchorMatchRate(anchorEntries).get(groupKey) ?? NaN,
      feedbackDistribution: feedbackCountDistribution(countEntries).get(groupKey) ?? { zero: 0, total: 0 },
      categoryCompliance: categoryComplianceRate(categories),
    };

    preview = sets
      .map((s) => ({ documentId: s.documentId, refId: refIdByDoc.get(s.documentId) ?? s.documentId, setId: s.id }))
      .toSorted((a, b) => a.refId.localeCompare(b.refId))
      .slice(0, 3)
      .map((s) => ({
        documentId: s.documentId,
        refId: s.refId,
        feedbacks: feedbacks
          .filter((f) => f.setId === s.setId)
          .toSorted((a, b) => a.ord - b.ord)
          .map((f) => ({ id: f.id, category: f.category, body: f.body, matchStart: f.matchStart })),
      }));
  }

  return { run, docs, variantLabel, summary, preview };
};

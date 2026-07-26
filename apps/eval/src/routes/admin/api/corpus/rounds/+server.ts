import { error, json } from '@sveltejs/kit';
import { and, asc, desc, eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { generateAbsoluteTasks, generateConfirmationTasks, generateScreeningTasks } from '$lib/domain/rounds.ts';
import { corpusRoundPayloadSchema } from '$lib/server/corpus-round-schemas.ts';
import { createDb, Documents, FeedbackSets, PipelineRuns, Rounds, Tasks, Variants } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import type { NewTask } from '$lib/domain/rounds.ts';
import type { RequestHandler } from './$types';

type Db = ReturnType<typeof createDb>;

// variant 라벨 → variants → 해당 라벨의 가장 최근 succeeded 실행 → feedback_sets(documentId → setId).
// 구 파이프라인과 재설계 파이프라인(analysis)은 실행 종류만 다르고 세트 저장 형태가 같아 함께 본다.
const resolveLabelSets = async (db: Db, label: string, corpusVersion: string): Promise<Map<string, string> | null> => {
  const [variant] = await db.select({ id: Variants.id }).from(Variants).where(eq(Variants.label, label)).limit(1);
  if (!variant) return null;

  const [latestRun] = await db
    .select({ id: PipelineRuns.id })
    .from(PipelineRuns)
    .where(
      and(
        inArray(PipelineRuns.kind, ['pipeline', 'analysis']),
        eq(PipelineRuns.variantId, variant.id),
        eq(PipelineRuns.corpusVersion, corpusVersion),
        eq(PipelineRuns.status, 'succeeded'),
      ),
    )
    .orderBy(desc(PipelineRuns.createdAt))
    .limit(1);
  if (!latestRun) return null;

  const sets = await db
    .select({ documentId: FeedbackSets.documentId, id: FeedbackSets.id })
    .from(FeedbackSets)
    .where(eq(FeedbackSets.runId, latestRun.id));
  return new Map(sets.map((s) => [s.documentId, s.id]));
};

const requireLabelSets = async (db: Db, label: string, corpusVersion: string): Promise<Map<string, string>> => {
  const sets = await resolveLabelSets(db, label, corpusVersion);
  if (!sets) {
    error(400, `no succeeded run for variant label: ${label} (corpus ${corpusVersion})`);
  }
  return sets;
};

// 절대평가는 라벨의 succeeded 실행을 전부 합쳐 문서 목록을 만든다 — 같은 후보를 여러 번 나눠
// 돌리는 것이 정상이라 "가장 최근 실행" 하나만 보면 앞서 돌린 문서들이 통째로 빠진다.
// 같은 문서가 두 실행에 있으면 나중 실행의 세트를 쓰고, 총평이 없는 세트(구 파이프라인 산출물)는
// 제외한다 — 재설계 파이프라인 초기 실행이 kind='pipeline'으로 기록된 것이 있어 kind로는 가릴 수 없다.
const requireAnalysisSets = async (db: Db, label: string, corpusVersion: string): Promise<Map<string, string>> => {
  const [variant] = await db.select({ id: Variants.id }).from(Variants).where(eq(Variants.label, label)).limit(1);
  if (!variant) {
    error(400, `unknown variant label: ${label}`);
  }

  const runs = await db
    .select({ id: PipelineRuns.id })
    .from(PipelineRuns)
    .where(and(eq(PipelineRuns.variantId, variant.id), eq(PipelineRuns.corpusVersion, corpusVersion), eq(PipelineRuns.status, 'succeeded')))
    .orderBy(asc(PipelineRuns.createdAt));
  if (runs.length === 0) {
    error(400, `no succeeded run for variant label: ${label} (corpus ${corpusVersion})`);
  }

  const rank = new Map(runs.map((run, i) => [run.id, i]));
  const sets = await db
    .select({ id: FeedbackSets.id, runId: FeedbackSets.runId, documentId: FeedbackSets.documentId, review: FeedbackSets.review })
    .from(FeedbackSets)
    .where(
      inArray(
        FeedbackSets.runId,
        runs.map((run) => run.id),
      ),
    );

  const latest = new Map<string, { setId: string; rank: number }>();
  for (const set of sets) {
    if (set.review === null) continue;
    const order = rank.get(set.runId) ?? -1;
    const current = latest.get(set.documentId);
    if (!current || order > current.rank) {
      latest.set(set.documentId, { setId: set.id, rank: order });
    }
  }
  if (latest.size === 0) {
    error(400, `no analysis feedback sets for variant label: ${label} (corpus ${corpusVersion})`);
  }

  return new Map([...latest].map(([documentId, v]) => [documentId, v.setId]));
};

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = corpusRoundPayloadSchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const payload = parsed.data;

  const [existing] = await db.select({ id: Rounds.id }).from(Rounds).where(eq(Rounds.id, payload.roundId));
  if (existing) {
    return json({ created: false, taskCount: 0 });
  }

  let newTasks: NewTask[];
  let roundConfig: Record<string, unknown>;

  if (payload.stage === 'screening') {
    if (!payload.variantLabels.includes(payload.baselineLabel)) {
      error(400, 'baselineLabel must be one of variantLabels');
    }
    const labelSets = await Promise.all(payload.variantLabels.map((label) => requireLabelSets(db, label, payload.corpusVersion)));
    const corpusDocs = await db.select({ id: Documents.id }).from(Documents).where(eq(Documents.corpusVersion, payload.corpusVersion));

    const documents = corpusDocs
      .map((doc) => ({ documentId: doc.id, setIds: labelSets.map((m) => m.get(doc.id)).filter((s): s is string => !!s) }))
      .filter((d) => d.setIds.length >= 2);
    if (documents.length === 0) {
      error(400, 'no documents with at least 2 matching feedback sets');
    }

    newTasks = generateScreeningTasks(documents, {
      overlapRatio: payload.overlapRatio,
      rng: Math.random,
    });
    roundConfig = {
      overlapRatio: payload.overlapRatio,
      baselineLabel: payload.baselineLabel,
      ...(payload.expectedEvaluators && { expectedEvaluators: payload.expectedEvaluators }),
    };
  } else if (payload.stage === 'absolute') {
    const labelSets = await requireAnalysisSets(db, payload.label, payload.corpusVersion);
    const documentIds = payload.documentIds ?? [...labelSets.keys()];

    const documents = documentIds
      .map((documentId) => {
        const setId = labelSets.get(documentId);
        return setId ? { documentId, setId } : null;
      })
      .filter((d): d is { documentId: string; setId: string } => d !== null);
    if (documents.length === 0) {
      error(400, 'no documents with a matching feedback set');
    }

    newTasks = generateAbsoluteTasks(documents, {
      requiredJudgments: payload.requiredJudgments,
      overlapRatio: payload.overlapRatio,
      rng: Math.random,
    });
    roundConfig = {
      label: payload.label,
      requiredJudgments: payload.requiredJudgments,
      overlapRatio: payload.overlapRatio,
      ...(payload.expectedEvaluators && { expectedEvaluators: payload.expectedEvaluators }),
    };
  } else {
    const [v0Sets, candidateSets] = await Promise.all([
      requireLabelSets(db, payload.v0Label, payload.corpusVersion),
      requireLabelSets(db, payload.candidateLabel, payload.corpusVersion),
    ]);
    const documentIds = payload.documentIds ?? [...v0Sets.keys()].filter((id) => candidateSets.has(id));

    const documents = documentIds
      .map((documentId) => {
        const v0SetId = v0Sets.get(documentId);
        const candidateSetId = candidateSets.get(documentId);
        return v0SetId && candidateSetId ? { documentId, v0SetId, candidateSetId } : null;
      })
      .filter((d): d is { documentId: string; v0SetId: string; candidateSetId: string } => d !== null);
    if (documents.length === 0) {
      error(400, 'no documents with matching v0/candidate feedback sets');
    }

    newTasks = generateConfirmationTasks(documents, { rng: Math.random });
    roundConfig = { baselineLabel: payload.v0Label };
  }

  const roundInsert = db.insert(Rounds).values({ id: payload.roundId, stage: payload.stage, config: roundConfig });
  const taskInserts = newTasks.map((task) =>
    db.insert(Tasks).values({
      id: nanoid(),
      roundId: payload.roundId,
      kind: task.kind,
      documentId: task.documentId,
      setIds: task.setIds,
      requiredJudgments: task.requiredJudgments,
      golden: task.golden,
    }),
  );

  // Rounds insert + Tasks insert 전체를 하나의 D1 batch로 묶어 부분 실패 잔존을 없앤다.
  await db.batch([roundInsert, ...taskInserts]);

  return json({ created: true, taskCount: newTasks.length });
};

import { error, json } from '@sveltejs/kit';
import { and, desc, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { generateConfirmationTasks, generateScreeningTasks } from '$lib/domain/rounds.ts';
import { corpusRoundPayloadSchema } from '$lib/server/corpus-round-schemas.ts';
import { createDb, Documents, FeedbackSets, PipelineRuns, Rounds, Tasks, Variants } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import type { NewTask } from '$lib/domain/rounds.ts';
import type { RequestHandler } from './$types';

type Db = ReturnType<typeof createDb>;

// variant 라벨 → variants → 해당 라벨의 가장 최근 succeeded 파이프라인 run → feedback_sets(documentId → setId)
const resolveLabelSets = async (db: Db, label: string, corpusVersion: string): Promise<Map<string, string> | null> => {
  const [variant] = await db.select({ id: Variants.id }).from(Variants).where(eq(Variants.label, label)).limit(1);
  if (!variant) return null;

  const [latestRun] = await db
    .select({ id: PipelineRuns.id })
    .from(PipelineRuns)
    .where(
      and(
        eq(PipelineRuns.kind, 'pipeline'),
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
    error(400, `no succeeded pipeline run for variant label: ${label} (corpus ${corpusVersion})`);
  }
  return sets;
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
      sanityRatio: payload.sanityRatio,
      rng: Math.random,
    });
    roundConfig = { overlapRatio: payload.overlapRatio, sanityRatio: payload.sanityRatio };
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
    roundConfig = {};
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

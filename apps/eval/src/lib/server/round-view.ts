import { eq, inArray } from 'drizzle-orm';
import { Documents, inChunks, ItemAnchors, ItemLinks, JudgmentItems, Judgments, Rounds, RunItems, Runs, Tasks } from '../../../core/db.ts';
import { assembleItems } from './run-view.ts';
import type { Db } from '../../../core/db.ts';
import type { ViewItem } from './run-view.ts';

export type RoundJudgment = {
  id: string;
  taskId: string;
  runId: string;
  evaluatorEmail: string;
  draft: boolean;
  stage: number;
  payload: Record<string, unknown>;
  elapsedSeconds: number;
  createdAt: string;
  updatedAt: string;
  entries: { itemId: string; payload: Record<string, unknown> }[];
};

export type RoundRun = { id: string; taskId: string; refId: string; characterCount: number; items: ViewItem[] };

// 라운드의 원자료. 평가 방식과 무관한 다섯 가지만 담는다 — payload 안이 무엇인지는 코어가 모른다.
export type RoundView = {
  round: { id: string; label: string; evaluationId: string; active: boolean };
  runs: RoundRun[];
  judgments: RoundJudgment[];
};

export const loadRoundView = async (db: Db, roundId: string): Promise<RoundView | null> => {
  const [round] = await db.select().from(Rounds).where(eq(Rounds.id, roundId));
  if (!round) return null;

  const tasks = await db.select({ id: Tasks.id, runId: Tasks.runId }).from(Tasks).where(eq(Tasks.roundId, roundId));
  const taskIds = tasks.map((t) => t.id);
  const runIds = tasks.map((t) => t.runId);

  const runRows = await inChunks(runIds, (chunk) =>
    db
      .select({ id: Runs.id, refId: Documents.refId, characterCount: Documents.characterCount })
      .from(Runs)
      .leftJoin(Documents, eq(Documents.id, Runs.documentId))
      .where(inArray(Runs.id, chunk)),
  );

  const itemRows = await inChunks(runIds, (chunk) =>
    db
      .select({
        id: RunItems.id,
        runId: RunItems.runId,
        kind: RunItems.kind,
        ord: RunItems.ord,
        body: RunItems.body,
        facets: RunItems.facets,
      })
      .from(RunItems)
      .where(inArray(RunItems.runId, chunk)),
  );
  const itemIds = itemRows.map((i) => i.id);
  const anchors = await inChunks(itemIds, (chunk) =>
    db
      .select({
        itemId: ItemAnchors.itemId,
        ord: ItemAnchors.ord,
        startText: ItemAnchors.startText,
        endText: ItemAnchors.endText,
        matchStart: ItemAnchors.matchStart,
        matchEnd: ItemAnchors.matchEnd,
        note: ItemAnchors.note,
      })
      .from(ItemAnchors)
      .where(inArray(ItemAnchors.itemId, chunk)),
  );
  const links = await inChunks(itemIds, (chunk) => db.select().from(ItemLinks).where(inArray(ItemLinks.itemId, chunk)));

  const judgmentRows = await inChunks(taskIds, (chunk) => db.select().from(Judgments).where(inArray(Judgments.taskId, chunk)));
  const entryRows = await inChunks(
    judgmentRows.map((j) => j.id),
    (chunk) => db.select().from(JudgmentItems).where(inArray(JudgmentItems.judgmentId, chunk)),
  );

  const runOfTask = new Map(tasks.map((t) => [t.id, t.runId]));
  const taskOfRun = new Map(tasks.map((t) => [t.runId, t.id]));

  return {
    round: { id: round.id, label: round.label, evaluationId: round.evaluationId, active: round.active },
    runs: runRows.map((run) => ({
      id: run.id,
      taskId: taskOfRun.get(run.id) ?? '',
      refId: run.refId ?? run.id,
      characterCount: run.characterCount ?? 0,
      items: assembleItems(
        itemRows.filter((i) => i.runId === run.id),
        anchors,
        links,
      ),
    })),
    judgments: judgmentRows.map((j) => ({
      id: j.id,
      taskId: j.taskId,
      runId: runOfTask.get(j.taskId) ?? '',
      evaluatorEmail: j.evaluatorEmail,
      draft: j.draft,
      stage: j.stage,
      payload: j.payload,
      elapsedSeconds: j.elapsedSeconds,
      createdAt: j.createdAt.toISOString(),
      updatedAt: j.updatedAt.toISOString(),
      entries: entryRows.filter((e) => e.judgmentId === j.id).map((e) => ({ itemId: e.itemId, payload: e.payload })),
    })),
  };
};

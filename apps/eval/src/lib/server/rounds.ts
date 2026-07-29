import { desc, eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { Documents, inChunks, Judgments, PromptSets, Rounds, Runs, TaskReleases, Tasks } from '../../../core/db.ts';
import { evaluationById } from '../../../core/registry.ts';
import type { Db, DocumentKind, RunStatus } from '../../../core/db.ts';

export type RoundCard = {
  id: string;
  label: string;
  evaluationId: string;
  total: number;
  done: number;
  mine: number;
  claimable: number;
  // 평가자에게 예고하는 분량. 세대·코퍼스마다 다르므로 문구에 수치를 박지 않고 데이터에서 조립한다.
  manuscript: { min: number; max: number; avg: number } | null;
};

export type RoundRun = {
  id: string;
  status: RunStatus;
  generationId: string | null;
  documentId: string;
  documentKind: DocumentKind | null;
  refId: string | null;
};

// 이미 라운드에 들어간 문서. 실행이 아니라 문서로 센다 — 같은 원고를 다시 돌리면 실행 id가
// 새로 나므로 실행으로 세면 같은 글이 다음 라운드에 그대로 다시 들어간다.
export const roundedDocumentIds = async (db: Db): Promise<string[]> => {
  const rows = await db.select({ documentId: Runs.documentId }).from(Tasks).innerJoin(Runs, eq(Runs.id, Tasks.runId));
  return [...new Set(rows.map((r) => r.documentId))];
};

const nameOf = (run: RoundRun): string => run.refId ?? run.id;

// 라운드에 넣을 수 있는 실행인지 판정한다. 화면이 이미 후보를 걸러 내보내므로 여기 걸리는 것은
// 폼을 직접 던졌거나, 두 라운드를 동시에 만들어 후보 목록이 낡은 경우다.
export const planRound = (input: {
  runs: RoundRun[];
  requestedIds: string[];
  generation: { id: string; label: string };
  usedDocumentIds: string[];
}): { ok: true } | { error: string } => {
  const { runs, requestedIds } = input;
  if (requestedIds.length === 0) return { error: '평가할 실행이 없습니다' };
  if (runs.length !== requestedIds.length) {
    return { error: `실행을 찾을 수 없습니다: ${requestedIds.filter((id) => runs.every((r) => r.id !== id)).join(', ')}` };
  }

  const notDone = runs.filter((r) => r.status !== 'done');
  if (notDone.length > 0) return { error: `완료되지 않은 실행이 있습니다: ${notDone.map((r) => r.id).join(', ')}` };

  // 라운드가 담는 실행들은 한 세대여야 한다 — 평가 정의가 그 세대의 산출물 모양을 전제하기 때문.
  const mismatched = runs.filter((r) => r.generationId !== input.generation.id);
  if (mismatched.length > 0) {
    return { error: `${input.generation.label} 평가에 다른 세대의 실행이 섞였습니다: ${mismatched.map((r) => r.id).join(', ')}` };
  }

  // 반입 문서는 공개 관문을 지나지 않았다. 표식이 없는 문서(kind null)도 여기서 걸린다.
  const intake = runs.filter((r) => r.documentKind !== 'sampled');
  if (intake.length > 0) {
    return { error: `반입 문서는 라운드에 넣을 수 없습니다: ${intake.map(nameOf).join(', ')}` };
  }

  const used = new Set(input.usedDocumentIds);
  const reused = runs.filter((r) => used.has(r.documentId));
  if (reused.length > 0) {
    return { error: `이미 다른 라운드에 쓰인 문서입니다: ${reused.map(nameOf).join(', ')}` };
  }

  // 한 라운드 안에서도 문서는 한 번만 나온다 — 같은 원고의 실행 둘을 넣으면 평가자가 같은 글을
  // 두 번 읽는다.
  const seen = new Set<string>();
  const twice: RoundRun[] = [];
  for (const run of runs) {
    if (seen.has(run.documentId)) twice.push(run);
    else seen.add(run.documentId);
  }
  if (twice.length > 0) {
    return { error: `같은 문서의 실행이 둘 이상 섞였습니다: ${twice.map(nameOf).join(', ')}` };
  }

  return { ok: true };
};

export const createRound = async (
  db: Db,
  input: { label: string; evaluationId: string; runIds: string[] },
): Promise<{ roundId: string } | { error: string }> => {
  const resolved = evaluationById(input.evaluationId);
  if (!resolved) return { error: `알 수 없는 평가 방식: ${input.evaluationId}` };

  const runs = await inChunks(input.runIds, (chunk) =>
    db
      .select({
        id: Runs.id,
        status: Runs.status,
        generationId: PromptSets.generationId,
        documentId: Runs.documentId,
        documentKind: Documents.kind,
        refId: Documents.refId,
      })
      .from(Runs)
      .leftJoin(PromptSets, eq(PromptSets.id, Runs.promptSetId))
      .leftJoin(Documents, eq(Documents.id, Runs.documentId))
      .where(inArray(Runs.id, chunk)),
  );

  const gate = planRound({
    runs,
    requestedIds: input.runIds,
    generation: { id: resolved.generation.id, label: resolved.generation.label },
    usedDocumentIds: await roundedDocumentIds(db),
  });
  if ('error' in gate) return gate;

  const roundId = nanoid();
  await db.insert(Rounds).values({ id: roundId, label: input.label, evaluationId: input.evaluationId, active: false });
  for (const run of runs) {
    await db.insert(Tasks).values({ id: nanoid(), roundId, runId: run.id });
  }
  return { roundId };
};

export const setRoundActive = async (db: Db, roundId: string, active: boolean): Promise<void> => {
  await db.update(Rounds).set({ active }).where(eq(Rounds.id, roundId));
};

const cardFor = async (db: Db, round: { id: string; label: string; evaluationId: string }, email: string): Promise<RoundCard> => {
  const tasks = await db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, round.id));
  const ids = tasks.map((t) => t.id);
  const judgments =
    ids.length > 0
      ? await inChunks(ids, (chunk) =>
          db
            .select({ taskId: Judgments.taskId, evaluatorEmail: Judgments.evaluatorEmail, draft: Judgments.draft })
            .from(Judgments)
            .where(inArray(Judgments.taskId, chunk)),
        )
      : [];
  const released =
    ids.length > 0
      ? await inChunks(ids, (chunk) =>
          db.select({ taskId: TaskReleases.taskId }).from(TaskReleases).where(inArray(TaskReleases.taskId, chunk)),
        )
      : [];

  const documents = await db
    .select({ characterCount: Documents.characterCount })
    .from(Tasks)
    .innerJoin(Runs, eq(Runs.id, Tasks.runId))
    .innerJoin(Documents, eq(Documents.id, Runs.documentId))
    .where(eq(Tasks.roundId, round.id));
  const counts = documents.map((d) => d.characterCount);

  const taken = new Set(judgments.map((j) => j.taskId));
  const myReleased = new Set(released.map((r) => r.taskId));
  return {
    id: round.id,
    label: round.label,
    evaluationId: round.evaluationId,
    total: tasks.length,
    done: judgments.filter((j) => !j.draft).length,
    mine: judgments.filter((j) => j.evaluatorEmail === email && !j.draft).length,
    claimable: tasks.filter((t) => !taken.has(t.id) && !myReleased.has(t.id)).length,
    manuscript:
      counts.length > 0
        ? {
            min: Math.min(...counts),
            max: Math.max(...counts),
            avg: Math.round(counts.reduce((a, b) => a + b, 0) / counts.length),
          }
        : null,
  };
};

// 활성 라운드는 여럿일 수 있다. 구 구조는 created_at 최신 하나를 라이브로 쳐서, 열람용 라운드를
// 넣는 것만으로 진행 중인 라운드가 탈취됐다.
export const activeRounds = async (db: Db, email: string): Promise<RoundCard[]> => {
  const rounds = await db
    .select({ id: Rounds.id, label: Rounds.label, evaluationId: Rounds.evaluationId })
    .from(Rounds)
    .where(eq(Rounds.active, true))
    .orderBy(desc(Rounds.createdAt));

  const cards: RoundCard[] = [];
  for (const round of rounds) cards.push(await cardFor(db, round, email));
  return cards;
};

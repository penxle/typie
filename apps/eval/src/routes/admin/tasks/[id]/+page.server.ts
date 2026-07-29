import { error } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { loadArtifacts } from '$lib/server/artifacts.ts';
import { loadRunView } from '$lib/server/run-view.ts';
import { createDb, JudgmentItems, Judgments, Rounds, Tasks } from '../../../../../core/db.ts';
import type { PageServerLoad } from './$types';

// 평가자에게 보이는 화면 그대로 미리본다. 저장 액션을 두지 않는 것이 곧 안전장치다 —
// 조작은 되지만 어디에도 남지 않는다.
export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const [task] = await db.select().from(Tasks).where(eq(Tasks.id, params.id));
  if (!task) error(404, 'task not found');
  const [round] = await db.select().from(Rounds).where(eq(Rounds.id, task.roundId));
  if (!round) error(500, 'round missing');

  const view = await loadRunView(db, task.runId);
  if (!view) error(500, 'run missing');

  // 이미 매겨진 판정이 있으면 그대로 얹어 보여준다 — 무엇을 보고 무엇을 답했는지가 같이 읽혀야 한다.
  const [judgment] = await db.select().from(Judgments).where(eq(Judgments.taskId, task.id));
  const entries = judgment ? await db.select().from(JudgmentItems).where(eq(JudgmentItems.judgmentId, judgment.id)) : [];

  return {
    evaluationId: round.evaluationId,
    round: { id: round.id, label: round.label },
    view,
    answers: Object.fromEntries(entries.map((e) => [e.itemId, e.payload])),
    runAnswer: judgment?.payload ?? {},
    evaluator: judgment?.evaluatorEmail ?? null,
    artifacts: await loadArtifacts(db, view.generationId, task.runId),
  };
};

import { error, fail, redirect } from '@sveltejs/kit';
import { and, eq } from 'drizzle-orm';
import { claimTask, isEvaluator } from '$lib/server/assign.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { activeRounds } from '$lib/server/rounds.ts';
import { createDb, Evaluators, Judgments, Tasks } from '../../core/db.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform, locals }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const [consent] = await db.select().from(Evaluators).where(eq(Evaluators.email, locals.email));
  if (!consent) redirect(302, '/consent');

  // 받아 놓고 아직 제출하지 않은 것들. draft로 좁히지 않으면 제출한 태스크가 계속 "이어서 하기"로
  // 남아 새 배정을 막는다.
  const drafts = await db
    .select({ taskId: Judgments.taskId, roundId: Tasks.roundId })
    .from(Judgments)
    .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, true)));

  return {
    email: locals.email,
    isAdmin: isAdmin(platform.env, locals.email),
    evaluating: consent.evaluating,
    rounds: await activeRounds(db, locals.email),
    drafts,
  };
};

export const actions: Actions = {
  claim: async ({ request, platform, locals }) => {
    if (!platform) error(500, 'platform unavailable');
    const db = createDb(platform.env.DB);
    const form = await request.formData();
    // 자격이 없어도 오류 화면을 띄우지 않는다 — 버튼이 보이지 않아야 하는 상태이므로 안내만 남긴다.
    if (!(await isEvaluator(db, locals.email))) return fail(403, { message: '아직 평가자로 등록되지 않았습니다' });

    const roundId = String(form.get('roundId') ?? '');
    const taskId = await claimTask(db, roundId, locals.email);
    if (!taskId) redirect(303, '/?empty=1');
    redirect(303, `/tasks/${taskId}`);
  },
};

import { error } from '@sveltejs/kit';
import { and, eq } from 'drizzle-orm';
import { claimableSummary } from '$lib/server/claim.ts';
import { createDb, Judgments, Tasks } from '$lib/server/db/index.ts';
import { effectiveProgress } from '$lib/server/progress.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform, locals }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const drafts = await db
    .select({ taskId: Judgments.taskId, kind: Tasks.kind, updatedAt: Judgments.updatedAt })
    .from(Judgments)
    .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, true)));

  const done = await db
    .select({ taskId: Judgments.taskId })
    .from(Judgments)
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, false)));

  // "내 진행"과 "라운드 전체 진행"을 분리한다 — 태스크는 평가자들이 나눠 가지므로 전체 태스크
  // 수는 개인 목표가 아니다. 개인에게는 판정 건수·이어할 것·새로 받을 수 있는 것만 보여준다.
  const round = await effectiveProgress(db);
  const { remaining, potential, quota } = await claimableSummary(db, locals.email, platform.env.ADMIN_EMAILS ?? '');

  const isAdmin = (platform.env.ADMIN_EMAILS ?? '')
    .split(',')
    .map((e) => e.trim())
    .includes(locals.email);

  return {
    email: locals.email,
    isAdmin,
    drafts,
    doneCount: done.length,
    // 개인 진행 분모 — 지금 기준 내가 하게 될 총량. 남이 태스크를 가져가면 줄어드는 동적 값이다.
    myTotal: done.length + drafts.length + potential,
    round,
    remaining,
    quota,
  };
};

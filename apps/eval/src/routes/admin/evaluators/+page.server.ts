import { error } from '@sveltejs/kit';
import { desc, eq } from 'drizzle-orm';
import { effectiveContributions } from '$lib/domain/contributions.ts';
import { softCap } from '$lib/server/claim.ts';
import { createDb, EvaluatorConsents, Judgments, Rounds, Tasks } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const consents = await db.select({ email: EvaluatorConsents.email }).from(EvaluatorConsents);
  const rounds = await db.select().from(Rounds).orderBy(desc(Rounds.createdAt));

  const summaries = [];
  for (const round of rounds) {
    const tasks = await db
      .select({ id: Tasks.id, requiredJudgments: Tasks.requiredJudgments })
      .from(Tasks)
      .where(eq(Tasks.roundId, round.id));
    const requiredTotal = tasks.reduce((sum, t) => sum + (t.requiredJudgments ?? 1), 0);

    const judgments = await db
      .select({
        id: Judgments.id,
        taskId: Judgments.taskId,
        evaluatorEmail: Judgments.evaluatorEmail,
        draft: Judgments.draft,
        createdAt: Judgments.createdAt,
        updatedAt: Judgments.updatedAt,
      })
      .from(Judgments)
      .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
      .where(eq(Tasks.roundId, round.id));

    // 동의한 전원을 행으로 만든다 — 판정이 0건인 사람이 곧 독려 대상이다.
    const byEmail = new Map<string, { confirmed: number; hasDraft: boolean; lastAt: Date | null }>();
    const ensure = (email: string) => {
      const row = byEmail.get(email) ?? { confirmed: 0, hasDraft: false, lastAt: null };
      byEmail.set(email, row);
      return row;
    };
    for (const judgment of judgments) {
      const row = ensure(judgment.evaluatorEmail);
      if (judgment.draft) row.hasDraft = true;
      else row.confirmed += 1;
      if (!row.lastAt || judgment.updatedAt > row.lastAt) row.lastAt = judgment.updatedAt;
    }
    for (const consent of consents) ensure(consent.email);

    const confirmedTotal = byEmail.values().reduce((sum, row) => sum + row.confirmed, 0);

    // 유효 판정 = 태스크별 min(확정 수, 필요 수) — 초과 배정 잉여를 뺀 실제 커버리지.
    const confirmedRows = judgments.filter((j) => !j.draft);
    const effectiveByEmail = effectiveContributions(tasks, confirmedRows);
    const effectiveTotal = effectiveByEmail.values().reduce((sum, n) => sum + n, 0);

    // 캡은 잉여로 소모된 용량을 반영해 자동 확대된다(claim.ts와 동일 산식). 개인 크레딧은 원시 확정 수.
    const surplusTotal = confirmedTotal - effectiveTotal;
    const expected = (round.config as { expectedEvaluators?: number } | null)?.expectedEvaluators;
    const cap =
      typeof expected === 'number' && Number.isSafeInteger(expected) && expected >= 1
        ? softCap(requiredTotal + surplusTotal, expected)
        : null;

    const evaluators = [...byEmail]
      .map(([email, row]) => ({
        email,
        confirmed: row.confirmed,
        effective: effectiveByEmail.get(email) ?? 0,
        hasDraft: row.hasDraft,
        lastAt: row.lastAt?.toISOString() ?? null,
        quotaLeft: cap === null ? null : Math.max(0, cap - row.confirmed),
      }))
      .toSorted((a, b) => b.confirmed - a.confirmed || a.email.localeCompare(b.email));

    summaries.push({
      roundId: round.id,
      stage: round.stage,
      requiredTotal,
      confirmedTotal,
      effectiveTotal,
      cap,
      expected: cap === null ? null : (expected ?? null),
      evaluators,
    });
  }

  return { summaries };
};

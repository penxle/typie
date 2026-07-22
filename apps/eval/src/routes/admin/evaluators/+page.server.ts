import { error } from '@sveltejs/kit';
import { desc, eq } from 'drizzle-orm';
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
      .select({ evaluatorEmail: Judgments.evaluatorEmail, draft: Judgments.draft, updatedAt: Judgments.updatedAt })
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

    const expected = (round.config as { expectedEvaluators?: number } | null)?.expectedEvaluators;
    const cap = typeof expected === 'number' && Number.isSafeInteger(expected) && expected >= 1 ? softCap(requiredTotal, expected) : null;
    const confirmedTotal = byEmail.values().reduce((sum, row) => sum + row.confirmed, 0);

    const evaluators = [...byEmail]
      .map(([email, row]) => ({
        email,
        confirmed: row.confirmed,
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
      cap,
      expected: cap === null ? null : (expected ?? null),
      evaluators,
    });
  }

  return { summaries };
};

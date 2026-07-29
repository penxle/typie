import { error } from '@sveltejs/kit';
import { desc, eq, inArray } from 'drizzle-orm';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, Evaluators, inChunks, Judgments, Rounds, Tasks } from '../../../../core/db.ts';
import { evaluationById } from '../../../../core/registry.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const evaluators = await db.select().from(Evaluators).orderBy(desc(Evaluators.consentedAt));
  const emails = evaluators.map((e) => e.email);
  const rounds = await db.select().from(Rounds).orderBy(desc(Rounds.createdAt));
  const tasks = await db.select({ id: Tasks.id, roundId: Tasks.roundId }).from(Tasks);
  const taskIds = tasks.map((t) => t.id);
  const judgments = await inChunks(taskIds, (chunk) =>
    db
      .select({
        taskId: Judgments.taskId,
        evaluatorEmail: Judgments.evaluatorEmail,
        draft: Judgments.draft,
        updatedAt: Judgments.updatedAt,
      })
      .from(Judgments)
      .where(inArray(Judgments.taskId, chunk)),
  );
  const roundOf = new Map(tasks.map((t) => [t.id, t.roundId]));

  return {
    roster: evaluators.map((e) => ({
      email: e.email,
      evaluating: e.evaluating,
      admin: isAdmin(platform.env, e.email),
    })),
    summaries: rounds.map((round) => {
      const mine = judgments.filter((j) => roundOf.get(j.taskId) === round.id);
      const participants = emails.filter(
        (email) => mine.some((j) => j.evaluatorEmail === email) || evaluators.some((e) => e.email === email && e.evaluating),
      );
      return {
        roundId: round.id,
        label: round.label,
        active: round.active,
        evaluationLabel: evaluationById(round.evaluationId)?.evaluation.label ?? round.evaluationId,
        taskTotal: tasks.filter((t) => t.roundId === round.id).length,
        confirmedTotal: mine.filter((j) => !j.draft).length,
        evaluators: participants.map((email) => {
          const own = mine.filter((j) => j.evaluatorEmail === email);
          const lastAt = own.reduce<Date | null>((latest, j) => (latest === null || j.updatedAt > latest ? j.updatedAt : latest), null);
          return {
            email,
            confirmed: own.filter((j) => !j.draft).length,
            hasDraft: own.some((j) => j.draft),
            lastAt: lastAt?.toISOString() ?? null,
          };
        }),
      };
    }),
  };
};

export const actions: Actions = {
  participation: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    await createDb(platform.env.DB)
      .update(Evaluators)
      .set({ evaluating: form.get('evaluating') === 'true' })
      .where(eq(Evaluators.email, String(form.get('email') ?? '')));
    return { ok: true };
  },
};

import { error, json } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { generateConfirmationTasks, generateScreeningTasks } from '$lib/domain/rounds.ts';
import { createDb, Rounds, Tasks } from '$lib/server/db/index.ts';
import { roundPayloadSchema } from '$lib/server/round-schemas.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = roundPayloadSchema.safeParse(await request.json());
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

  const newTasks =
    payload.stage === 'screening'
      ? generateScreeningTasks(payload.documents, {
          overlapRatio: payload.overlapRatio,
          rng: Math.random,
        })
      : generateConfirmationTasks(payload.documents, { rng: Math.random });

  await db.insert(Rounds).values({
    id: payload.roundId,
    stage: payload.stage,
    config: payload.stage === 'screening' ? { overlapRatio: payload.overlapRatio } : {},
  });

  for (const task of newTasks) {
    await db.insert(Tasks).values({
      id: nanoid(),
      roundId: payload.roundId,
      kind: task.kind,
      documentId: task.documentId,
      setIds: task.setIds,
      requiredJudgments: task.requiredJudgments,
      golden: task.golden,
    });
  }

  return json({ created: true, taskCount: newTasks.length });
};

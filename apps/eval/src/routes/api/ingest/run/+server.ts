import { error, json } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDb, Feedbacks, FeedbackSets, Runs, Variants } from '$lib/server/db/index.ts';
import { runPayloadSchema } from '$lib/server/ingest-schemas.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = runPayloadSchema.safeParse(await request.json());
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const payload = parsed.data;

  const [existingRun] = await db.select({ id: Runs.id }).from(Runs).where(eq(Runs.id, payload.runId));
  if (existingRun) {
    const existingSets = await db
      .select({ documentId: FeedbackSets.documentId, setId: FeedbackSets.id })
      .from(FeedbackSets)
      .where(eq(FeedbackSets.runId, payload.runId));
    return json({ created: false, sets: existingSets });
  }

  let [variant] = await db.select().from(Variants).where(eq(Variants.label, payload.variantLabel));
  if (!variant) {
    [variant] = await db.insert(Variants).values({ id: nanoid(), label: payload.variantLabel, round: payload.round }).returning();
  }

  await db.insert(Runs).values({
    id: payload.runId,
    variantId: variant.id,
    corpusVersion: payload.corpusVersion,
    meta: payload.meta,
  });

  const createdSets: { documentId: string; setId: string }[] = [];
  for (const set of payload.sets) {
    const setId = nanoid();
    createdSets.push({ documentId: set.documentId, setId });
    await db.insert(FeedbackSets).values({
      id: setId,
      runId: payload.runId,
      documentId: set.documentId,
      variantId: variant.id,
    });
    for (const [ord, feedback] of set.feedbacks.entries()) {
      await db.insert(Feedbacks).values({
        id: nanoid(),
        setId,
        ord,
        startText: feedback.startText,
        endText: feedback.endText,
        matchStart: feedback.matchStart,
        matchEnd: feedback.matchEnd,
        category: feedback.category,
        body: feedback.body,
      });
    }
  }

  return json({ created: true, sets: createdSets });
};

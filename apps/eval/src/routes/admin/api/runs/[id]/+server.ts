import { error, json } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, PipelineRunDocs, PipelineRuns } from '$lib/server/db/index.ts';
import { refreshRun } from '$lib/server/runs.ts';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  await refreshRun(db, platform.env, params.id);

  const [run] = await db.select().from(PipelineRuns).where(eq(PipelineRuns.id, params.id)).limit(1);
  if (!run) {
    error(404, 'run not found');
  }

  const docs = run.kind === 'pipeline' ? await db.select().from(PipelineRunDocs).where(eq(PipelineRunDocs.runId, params.id)) : [];

  return json({ run, docs });
};

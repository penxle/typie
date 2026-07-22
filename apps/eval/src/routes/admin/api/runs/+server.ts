import { error, json } from '@sveltejs/kit';
import { desc } from 'drizzle-orm';
import { createDb, PipelineRuns } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import { runCreateSchema } from '$lib/server/run-schemas.ts';
import { refreshRun, spawnPipelineRun, spawnSamplingRun } from '$lib/server/runs.ts';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const runs = await db.select().from(PipelineRuns).orderBy(desc(PipelineRuns.createdAt)).limit(100);
  await Promise.all(runs.filter((run) => run.status === 'running').map((run) => refreshRun(db, platform.env, run.id)));

  const refreshedRuns = await db.select().from(PipelineRuns).orderBy(desc(PipelineRuns.createdAt)).limit(100);
  return json({ runs: refreshedRuns });
};

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = runCreateSchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const body = parsed.data;

  if (body.kind === 'sampling') {
    const result = await spawnSamplingRun(db, platform.env, { corpusVersion: body.corpusVersion, size: body.size });
    return json(result);
  }

  const result = await spawnPipelineRun(db, platform.env, { promptVariantId: body.promptVariantId, corpusVersion: body.corpusVersion });
  if ('error' in result) {
    error(400, result.error);
  }

  return json(result);
};

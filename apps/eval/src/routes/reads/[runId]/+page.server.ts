import { error } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { loadArtifacts } from '$lib/server/artifacts.ts';
import { loadRunView } from '$lib/server/run-view.ts';
import { createDb, Runs } from '../../../../core/db.ts';
import type { PageServerLoad } from './$types';

// 어드민 밖 열람 경로. 작가에게 결과를 보여주는 자리라 가드를 두지 않는다(Access 통과가 전부).
export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const [run] = await db.select({ status: Runs.status }).from(Runs).where(eq(Runs.id, params.runId));
  if (!run) error(404, 'run not found');

  const view = await loadRunView(db, params.runId);
  if (!view) error(404, 'run not found');

  return { done: run.status === 'done', view, artifacts: await loadArtifacts(db, view.generationId, params.runId) };
};

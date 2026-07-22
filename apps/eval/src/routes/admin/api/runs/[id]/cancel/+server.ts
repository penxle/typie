import { error, json } from '@sveltejs/kit';
import { createDb } from '$lib/server/db/index.ts';
import { cancelRun } from '$lib/server/runs.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const result = await cancelRun(db, platform.env, params.id);
  if ('error' in result) {
    error(404, result.error);
  }

  return json(result);
};

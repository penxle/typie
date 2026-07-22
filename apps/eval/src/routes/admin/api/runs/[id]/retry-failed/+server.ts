import { error, json } from '@sveltejs/kit';
import { createDb } from '$lib/server/db/index.ts';
import { retryFailedDocs } from '$lib/server/runs.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const result = await retryFailedDocs(db, platform.env, params.id);
  if ('error' in result) {
    error(400, result.error);
  }

  return json(result);
};

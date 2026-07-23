import { error, json } from '@sveltejs/kit';
import { claimNextTask } from '$lib/server/claim.ts';
import { createDb } from '$lib/server/db/index.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ platform, locals }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }
  const db = createDb(platform.env.DB);
  const taskId = await claimNextTask(db, locals.email, platform.env.ADMIN_EMAILS ?? '');
  return json({ taskId });
};

import { error, json } from '@sveltejs/kit';
import { desc } from 'drizzle-orm';
import { createDb, PromptApplies } from '$lib/server/db/index.ts';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const applies = await db.select().from(PromptApplies).orderBy(desc(PromptApplies.createdAt)).limit(200);

  return json({ applies });
};

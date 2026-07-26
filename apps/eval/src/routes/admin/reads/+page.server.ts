import { error } from '@sveltejs/kit';
import { desc } from 'drizzle-orm';
import { AnalysisPromptSets, createDb } from '$lib/server/db/index.ts';
import { listPersonalReads } from '$lib/server/personal.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const sets = await db
    .select({ id: AnalysisPromptSets.id, label: AnalysisPromptSets.label })
    .from(AnalysisPromptSets)
    .orderBy(desc(AnalysisPromptSets.createdAt));

  return { promptSets: sets, reads: await listPersonalReads(db) };
};

import { error } from '@sveltejs/kit';
import { count, desc } from 'drizzle-orm';
import { createDb, Documents, PromptSets, Runs } from '../../../core/db.ts';
import { generationById } from '../../../core/registry.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const [documents] = await db.select({ n: count() }).from(Documents);
  const sets = await db
    .select({ id: PromptSets.id, label: PromptSets.label, generationId: PromptSets.generationId })
    .from(PromptSets)
    .orderBy(desc(PromptSets.createdAt));
  const runs = await db
    .select({ id: Runs.id, status: Runs.status, promptSetId: Runs.promptSetId })
    .from(Runs)
    .orderBy(desc(Runs.createdAt));

  const runningRuns = runs.filter((r) => r.status === 'running' || r.status === 'pending');
  const ranSetIds = new Set(runs.map((r) => r.promptSetId));

  const nextAction =
    sets.length === 0
      ? ({ kind: 'create-prompt-set' } as const)
      : runs.length === 0
        ? ({ kind: 'run' } as const)
        : runningRuns[0]
          ? ({ kind: 'view-run', runId: runningRuns[0].id } as const)
          : ({ kind: 'all-clear' } as const);

  return {
    documentCount: documents?.n ?? 0,
    runningCount: runningRuns.length,
    promptSetSummaries: sets.map((s) => ({
      id: s.id,
      label: s.label,
      generationLabel: generationById(s.generationId)?.label ?? s.generationId,
      status: ranSetIds.has(s.id) ? ('ran' as const) : ('draft' as const),
    })),
    nextAction,
  };
};

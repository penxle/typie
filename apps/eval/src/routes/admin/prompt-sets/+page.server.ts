import { error, fail, redirect } from '@sveltejs/kit';
import { desc } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDb, PromptSets, Runs } from '../../../../core/db.ts';
import { promptPhases } from '../../../../core/prompt-set.ts';
import { generationById, GENERATIONS } from '../../../../core/registry.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);
  const sets = await db.select().from(PromptSets).orderBy(desc(PromptSets.createdAt));
  const runs = await db.select({ promptSetId: Runs.promptSetId }).from(Runs);
  const ranSetIds = new Set(runs.map((r) => r.promptSetId));

  return {
    sets: sets.map((s) => ({
      id: s.id,
      label: s.label,
      note: s.note,
      generationId: s.generationId,
      generationLabel: generationById(s.generationId)?.label ?? `${s.generationId} (제거됨)`,
      status: ranSetIds.has(s.id) ? ('ran' as const) : ('draft' as const),
      createdAt: s.createdAt.toISOString(),
    })),
    generations: GENERATIONS.map((g) => ({
      id: g.id,
      label: g.label,
      status: g.status,
      phases: promptPhases(g).map((p) => ({ key: p.key, label: p.label })),
    })),
  };
};

export const actions: Actions = {
  create: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    const generationId = String(form.get('generationId') ?? '');
    const label = String(form.get('label') ?? '').trim();

    const manifest = generationById(generationId);
    if (!manifest) return fail(400, { message: '알 수 없는 세대입니다' });
    if (manifest.status !== 'active') return fail(400, { message: '동결된 세대에는 새 묶음을 만들 수 없습니다' });
    if (!label) return fail(400, { message: '라벨이 필요합니다' });

    // 빈 골격으로 만들고 상세에서 채운다 — 매니페스트가 단계를 정하므로 폼이 자동 생성된다.
    const content = Object.fromEntries(
      promptPhases(manifest).map((p) => [p.key, { system: '', model: 'anthropic/claude-opus-5', effort: null }]),
    );

    const id = nanoid();
    await createDb(platform.env.DB).insert(PromptSets).values({ id, generationId, label, content });
    redirect(303, `/admin/prompt-sets/${id}`);
  },
};

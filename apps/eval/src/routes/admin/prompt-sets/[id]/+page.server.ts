import { error, fail } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, PromptSets } from '../../../../../core/db.ts';
import { promptPhases, validatePromptSet } from '../../../../../core/prompt-set.ts';
import { generationById } from '../../../../../core/registry.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const [set] = await createDb(platform.env.DB).select().from(PromptSets).where(eq(PromptSets.id, params.id));
  if (!set) error(404, 'prompt set not found');

  const manifest = generationById(set.generationId);
  if (!manifest) {
    return {
      set: { id: set.id, label: set.label, note: set.note, generationId: set.generationId },
      phases: [],
      violations: [],
      frozen: true,
    };
  }

  const content = set.content as Record<string, { system?: string; model?: string; effort?: string | null }>;
  return {
    set: { id: set.id, label: set.label, note: set.note, generationId: set.generationId },
    phases: promptPhases(manifest).map((p) => ({
      key: p.key,
      label: p.label,
      system: content[p.key]?.system ?? '',
      model: content[p.key]?.model ?? '',
      effort: content[p.key]?.effort ?? '',
    })),
    violations: validatePromptSet(manifest, set.content),
    frozen: manifest.status !== 'active',
  };
};

export const actions: Actions = {
  save: async ({ params, request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const db = createDb(platform.env.DB);
    const [set] = await db.select().from(PromptSets).where(eq(PromptSets.id, params.id));
    if (!set) error(404, 'prompt set not found');

    const manifest = generationById(set.generationId);
    if (!manifest) return fail(400, { message: '세대 모듈이 제거되어 수정할 수 없습니다' });

    const form = await request.formData();
    const content = Object.fromEntries(
      promptPhases(manifest).map((p) => [
        p.key,
        {
          system: String(form.get(`${p.key}.system`) ?? ''),
          model: String(form.get(`${p.key}.model`) ?? ''),
          effort: String(form.get(`${p.key}.effort`) ?? '').trim() || null,
        },
      ]),
    );

    // 매니페스트에 없는 키와 빠진 단계는 저장이 거부된다.
    const violations = validatePromptSet(manifest, content);
    if (violations.length > 0) return fail(400, { message: violations.join(' / ') });

    await db
      .update(PromptSets)
      .set({ content, note: String(form.get('note') ?? '').trim() || null })
      .where(eq(PromptSets.id, params.id));
    return { saved: true };
  },
};

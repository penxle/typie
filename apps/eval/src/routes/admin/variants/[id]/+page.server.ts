import { error } from '@sveltejs/kit';
import { eq, sql } from 'drizzle-orm';
import { createDb, Documents, PromptVariants } from '$lib/server/db/index.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import type { VariantContent } from '$lib/domain/admin-types.ts';
import type { PageServerLoad } from './$types';

const emptyStagePrompt = () => ({ system: '', tools: {}, model: '', effort: null });

const emptyContent = (): VariantContent => ({
  summarize: emptyStagePrompt(),
  meta: emptyStagePrompt(),
  analyze: emptyStagePrompt(),
});

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const corpusVersionRows = await db
    .select({ corpusVersion: Documents.corpusVersion })
    .from(Documents)
    .groupBy(Documents.corpusVersion)
    .orderBy(sql`max(${Documents.createdAt}) desc`);
  const corpusVersions = corpusVersionRows.map((r) => r.corpusVersion);

  if (params.id === 'new') {
    const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);

    let content: VariantContent;
    let prefillError: string | null = null;
    try {
      content = await api.current();
    } catch (err) {
      content = emptyContent();
      prefillError = String(err).slice(0, 200);
    }

    return { isNew: true as const, variant: null, baseLabel: null, content, prefillError, corpusVersions };
  }

  const [variant] = await db.select().from(PromptVariants).where(eq(PromptVariants.id, params.id)).limit(1);
  if (!variant) {
    error(404, 'variant not found');
  }

  let baseLabel: string | null = null;
  if (variant.baseVariantId) {
    const [base] = await db
      .select({ label: PromptVariants.label })
      .from(PromptVariants)
      .where(eq(PromptVariants.id, variant.baseVariantId))
      .limit(1);
    baseLabel = base?.label ?? null;
  }

  return { isNew: false as const, variant, baseLabel, content: variant.content, prefillError: null, corpusVersions };
};

import { error, json } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { performApply } from '$lib/server/apply.ts';
import { applySchema } from '$lib/server/apply-schemas.ts';
import { createDb, PromptVariants } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform, locals }) => {
  const parsed = applySchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const body = parsed.data;

  const [variant] = await db
    .select({ content: PromptVariants.content })
    .from(PromptVariants)
    .where(eq(PromptVariants.id, body.promptVariantId))
    .limit(1);
  if (!variant) {
    error(404, 'variant not found');
  }

  const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);
  const result = await performApply(db, api, {
    appliedVariantId: body.promptVariantId,
    stage: body.stage,
    content: variant.content[body.stage],
    appliedBy: locals.email,
  });

  return json({ ok: result.ok });
};

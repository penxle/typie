import { error, json } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { checkRollbackAllowed, performApply } from '$lib/server/apply.ts';
import { rollbackSchema } from '$lib/server/apply-schemas.ts';
import { createDb, PromptApplies } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform, locals }) => {
  const parsed = rollbackSchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const [original] = await db.select().from(PromptApplies).where(eq(PromptApplies.id, parsed.data.applyId)).limit(1);
  if (!original) {
    error(404, 'apply not found');
  }

  const guard = await checkRollbackAllowed(db, original);
  if (!guard.ok) {
    return json({ error: guard.reason }, { status: 409 });
  }

  const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);
  const result = await performApply(db, api, {
    appliedVariantId: original.appliedVariantId,
    stage: original.appliedStage,
    content: original.prev,
    appliedBy: locals.email,
  });

  return json({ ok: result.ok });
};

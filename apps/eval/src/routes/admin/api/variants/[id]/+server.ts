import { error, json } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDb, PromptVariants } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import { variantCreateSchema } from '$lib/server/variant-schemas.ts';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const [variant] = await db.select().from(PromptVariants).where(eq(PromptVariants.id, params.id)).limit(1);
  if (!variant) {
    error(404, 'variant not found');
  }

  return json({ variant });
};

export const PUT: RequestHandler = async ({ params, request, platform }) => {
  const parsed = variantCreateSchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const body = parsed.data;

  const [base] = await db.select({ id: PromptVariants.id }).from(PromptVariants).where(eq(PromptVariants.id, params.id)).limit(1);
  if (!base) {
    error(404, 'variant not found');
  }

  const [existingLabel] = await db
    .select({ id: PromptVariants.id })
    .from(PromptVariants)
    .where(eq(PromptVariants.label, body.label))
    .limit(1);
  if (existingLabel) {
    error(409, 'label already exists');
  }

  const [variant] = await db
    .insert(PromptVariants)
    .values({ id: nanoid(), label: body.label, note: body.note ?? null, content: body.content, baseVariantId: params.id })
    .returning();

  return json({ variant });
};

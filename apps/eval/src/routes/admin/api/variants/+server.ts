import { error, json } from '@sveltejs/kit';
import { desc, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDb, PromptVariants } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import { variantCreateSchema } from '$lib/server/variant-schemas.ts';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const variants = await db.select().from(PromptVariants).orderBy(desc(PromptVariants.createdAt));

  return json({ variants });
};

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = variantCreateSchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const body = parsed.data;

  const [existing] = await db.select({ id: PromptVariants.id }).from(PromptVariants).where(eq(PromptVariants.label, body.label)).limit(1);
  if (existing) {
    error(409, 'label already exists');
  }

  const [variant] = await db
    .insert(PromptVariants)
    .values({ id: nanoid(), label: body.label, note: body.note ?? null, content: body.content })
    .returning();

  return json({ variant });
};

import { error, json } from '@sveltejs/kit';
import { inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDb, Documents } from '$lib/server/db/index.ts';
import { corpusPayloadSchema } from '$lib/server/ingest-schemas.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = corpusPayloadSchema.safeParse(await request.json());
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const payload = parsed.data;

  const existing = await db
    .select({ id: Documents.id })
    .from(Documents)
    .where(
      inArray(
        Documents.id,
        payload.documents.map((d) => d.id),
      ),
    );
  const existingIds = new Set(existing.map((d) => d.id));
  const fresh = payload.documents.filter((d) => !existingIds.has(d.id));

  for (const doc of fresh) {
    await db.insert(Documents).values({
      id: doc.id,
      refId: doc.refId,
      content: doc.content,
      characterCount: doc.characterCount,
      corpusVersion: payload.corpusVersion,
    });
  }

  return json({ inserted: fresh.length, skipped: existingIds.size, batchId: nanoid() });
};

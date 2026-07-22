import { error } from '@sveltejs/kit';
import { and, eq } from 'drizzle-orm';
import { createDb, Documents } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const [document] = await db
    .select()
    .from(Documents)
    .where(and(eq(Documents.corpusVersion, params.version), eq(Documents.id, params.id)))
    .limit(1);

  if (!document) {
    error(404, 'document not found');
  }

  return {
    corpusVersion: params.version,
    document: {
      id: document.id,
      refId: document.refId,
      content: document.content,
      characterCount: document.characterCount,
      lineBreakCount: (document.content.match(/\n/g) ?? []).length,
    },
  };
};

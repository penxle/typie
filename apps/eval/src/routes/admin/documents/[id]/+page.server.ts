import { error } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, Documents } from '../../../../../core/db.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) error(500, 'platform unavailable');

  const [document] = await createDb(platform.env.DB).select().from(Documents).where(eq(Documents.id, params.id));
  if (!document) error(404, 'document not found');

  return {
    document: {
      refId: document.refId,
      kind: document.kind,
      content: document.content,
      characterCount: document.characterCount,
      lineBreakCount: document.content.split('\n').length - 1,
    },
  };
};

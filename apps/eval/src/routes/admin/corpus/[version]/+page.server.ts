import { error } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, Documents } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const rows = await db
    .select({ id: Documents.id, refId: Documents.refId, content: Documents.content, characterCount: Documents.characterCount })
    .from(Documents)
    .where(eq(Documents.corpusVersion, params.version))
    .orderBy(Documents.refId);

  if (rows.length === 0) {
    error(404, 'corpus version not found');
  }

  // 개행 수는 스키마에 별도 컬럼이 없어 content에서 직접 계산한다 — 충실 추출 검증용.
  // content 자체는 여기서만 읽고 응답에는 담지 않아 목록 페이로드가 가벼워진다.
  const documents = rows.map((row) => ({
    id: row.id,
    refId: row.refId,
    characterCount: row.characterCount,
    lineBreakCount: (row.content.match(/\n/g) ?? []).length,
  }));

  return { corpusVersion: params.version, documents };
};

import { error } from '@sveltejs/kit';
import { sql } from 'drizzle-orm';
import { createDb, Documents } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  // sql<number>로 감싼 집계식은 drizzle의 timestamp 매핑을 거치지 않으므로 min(created_at)은
  // 원시 unix seconds로 온다 — 아래에서 직접 *1000 보정한다.
  // 목록에는 개인 열람용도 함께 둔다 — 실행 후보에서 뺐을 뿐 무엇이 들어와 있는지는 보여야 한다.
  const rows = await db
    .select({
      corpusVersion: Documents.corpusVersion,
      kind: Documents.kind,
      docCount: sql<number>`count(*)`,
      totalCharacters: sql<number>`sum(${Documents.characterCount})`,
      firstInsertedAtRaw: sql<number>`min(${Documents.createdAt})`,
    })
    .from(Documents)
    .groupBy(Documents.corpusVersion, Documents.kind)
    .orderBy(sql`min(${Documents.createdAt}) desc`);

  const versions = rows.map((row) => ({
    corpusVersion: row.corpusVersion,
    kind: row.kind,
    docCount: row.docCount,
    totalCharacters: row.totalCharacters,
    firstInsertedAt: new Date(row.firstInsertedAtRaw * 1000).toISOString(),
  }));

  return { versions };
};

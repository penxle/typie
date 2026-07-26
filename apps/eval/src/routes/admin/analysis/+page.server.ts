import { error } from '@sveltejs/kit';
import { desc, eq, sql } from 'drizzle-orm';
import { AnalysisPromptSets, createDb, Documents } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const sets = await db.select().from(AnalysisPromptSets).orderBy(desc(AnalysisPromptSets.createdAt));
  const corpusRows = await db
    .select({ version: Documents.corpusVersion, count: sql<number>`count(*)`, characters: sql<number>`sum(${Documents.characterCount})` })
    .from(Documents)
    // 개인 열람용 글은 평가 대상이 아니다. 첫 줄이 기본 선택이라 목록에 두면 새로 들일 때마다
    // 기본 선택을 가로채 30편짜리 코퍼스 대신 한 편짜리로 실행이 나간다.
    .where(eq(Documents.kind, 'corpus'))
    .groupBy(Documents.corpusVersion)
    // 가장 최근에 적재한 코퍼스가 먼저 오게 한다 — 목록 첫 줄이 기본 선택이라 순서가 곧 실수 방지책이다.
    .orderBy(sql`max(${Documents.createdAt}) desc`);

  return {
    sets: sets.map((set) => ({
      id: set.id,
      label: set.label,
      note: set.note,
      // 프롬프트 본문은 내려보내지 않는다 — 목록에서 필요한 건 어느 모델을 쓰는지와 분량뿐이다.
      stages: Object.entries(set.content).map(([stage, prompt]) => ({
        stage,
        model: prompt.model,
        effort: prompt.effort,
        length: prompt.system.length,
      })),
    })),
    corpusVersions: corpusRows.map((row) => ({ version: row.version, count: row.count, characters: row.characters })),
  };
};

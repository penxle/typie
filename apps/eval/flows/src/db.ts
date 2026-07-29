import { eq } from 'drizzle-orm';
import { drizzle } from 'drizzle-orm/d1';
import * as schema from '../../src/lib/server/db/schema.ts';
import { CHUNK_VERSION } from './text.ts';
import type { D1Database } from '@cloudflare/workers-types';

export const createDb = (d1: D1Database) => drizzle(d1, { schema });
export type Db = ReturnType<typeof createDb>;

export const {
  AnalysisStageUsage,
  Documents,
  Variants,
  PromptVariants,
  AnalysisPromptSets,
  PipelineRuns,
  PipelineRunDocs,
  FeedbackSets,
  Feedbacks,
  FeedbackAnchors,
  StageCache,
} = schema;

// 청크 index가 키에 들어가므로 청킹 규칙이 바뀌면 같은 index가 다른 본문을 가리킨다 —
// CHUNK_VERSION을 섞어 옛 요약이 조용히 재사용되는 것을 막는다.
export const summarizeCacheKey = (promptHash: string, documentId: string, index: number): string =>
  `summarize/v${CHUNK_VERSION}/${promptHash}-${documentId}-${index}`;

export const metaCacheKey = (summarizeHash: string, metaHash: string, documentId: string): string =>
  `meta/v${CHUNK_VERSION}/${summarizeHash}-${metaHash}-${documentId}`;

export const readStageCache = async <T>(db: Db, key: string): Promise<T | null> => {
  const [row] = await db.select({ value: StageCache.value }).from(StageCache).where(eq(StageCache.key, key)).limit(1);
  return row ? (row.value as T) : null;
};

export const writeStageCache = async (db: Db, key: string, value: unknown): Promise<void> => {
  await db.insert(StageCache).values({ key, value }).onConflictDoNothing();
};

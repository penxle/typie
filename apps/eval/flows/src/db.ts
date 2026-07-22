import { eq } from 'drizzle-orm';
import { drizzle } from 'drizzle-orm/d1';
import * as schema from '../../src/lib/server/db/schema.ts';
import type { D1Database } from '@cloudflare/workers-types';

export const createDb = (d1: D1Database) => drizzle(d1, { schema });
export type Db = ReturnType<typeof createDb>;

export const { Documents, Variants, PromptVariants, PipelineRuns, PipelineRunDocs, FeedbackSets, Feedbacks, StageCache } = schema;

export const summarizeCacheKey = (promptHash: string, documentId: string, index: number): string =>
  `summarize/${promptHash}-${documentId}-${index}`;

export const metaCacheKey = (summarizeHash: string, metaHash: string, documentId: string): string =>
  `meta/${summarizeHash}-${metaHash}-${documentId}`;

export const readStageCache = async <T>(db: Db, key: string): Promise<T | null> => {
  const [row] = await db.select({ value: StageCache.value }).from(StageCache).where(eq(StageCache.key, key)).limit(1);
  return row ? (row.value as T) : null;
};

export const writeStageCache = async (db: Db, key: string, value: unknown): Promise<void> => {
  await db.insert(StageCache).values({ key, value }).onConflictDoNothing();
};

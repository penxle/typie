import { and, eq } from 'drizzle-orm';
import { CallCache } from '../../core/db.ts';
import type { Db } from '../../core/db.ts';

export * from '../../core/db.ts';

export const readCache = async <T>(db: Db, runId: string, key: string): Promise<T | null> => {
  const [row] = await db
    .select({ value: CallCache.value })
    .from(CallCache)
    .where(and(eq(CallCache.runId, runId), eq(CallCache.key, key)))
    .limit(1);
  return row ? ((row.value as { value: T }).value ?? null) : null;
};

export const writeCache = async (db: Db, runId: string, key: string, value: unknown): Promise<void> => {
  await db.insert(CallCache).values({ runId, key, value: { value } }).onConflictDoNothing();
};

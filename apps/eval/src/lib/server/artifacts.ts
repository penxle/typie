import { and, eq, inArray } from 'drizzle-orm';
import { Ledgers } from '../../../core/db.ts';
import { generationById } from '../../../core/registry.ts';
import type { Db } from '../../../core/db.ts';

// 단계 산출물이 무엇인지는 세대가 안다 — 매니페스트의 선언대로 원장을 읽어 접는다.
export const loadArtifacts = async (
  db: Db,
  generationId: string | null,
  runId: string,
): Promise<{ label: string; value: unknown } | null> => {
  const spec = generationById(generationId ?? '')?.artifacts ?? null;
  if (!spec) return null;

  const rows = await db
    .select({ key: Ledgers.key, value: Ledgers.value })
    .from(Ledgers)
    .where(and(eq(Ledgers.runId, runId), inArray(Ledgers.key, spec.ledgerKeys)));
  const value = spec.select(Object.fromEntries(rows.map((r) => [r.key, r.value])));
  return value ? { label: spec.label, value } : null;
};

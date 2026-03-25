import { and, eq, gt, gte, lt, or } from 'drizzle-orm';
import { DocumentCharacterCountChanges } from '#/db/index.ts';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';

export async function hasBillableUsageDuring(db: Database | Transaction, userId: string, from: Dayjs, to: Dayjs): Promise<boolean> {
  const result = await db
    .select({ id: DocumentCharacterCountChanges.id })
    .from(DocumentCharacterCountChanges)
    .where(
      and(
        eq(DocumentCharacterCountChanges.userId, userId),
        gte(DocumentCharacterCountChanges.bucket, from),
        lt(DocumentCharacterCountChanges.bucket, to),
        or(gt(DocumentCharacterCountChanges.additions, 0), gt(DocumentCharacterCountChanges.deletions, 0)),
      ),
    )
    .limit(1)
    .then((rows) => rows.length > 0);

  return result;
}

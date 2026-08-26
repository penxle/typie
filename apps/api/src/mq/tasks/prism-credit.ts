import * as Sentry from '@sentry/node';
import { sql } from 'drizzle-orm';
import { dbr, PrismCreditEntries } from '#/db/index.ts';
import { opsAlert } from '#/utils/ops-alert.ts';
import { defineCron } from '../types.ts';
import type { SQL } from 'drizzle-orm';

const SAMPLE_LIMIT = 10;

type InvariantCheck = { key: string; violations: SQL };

const INVARIANT_CHECKS: InvariantCheck[] = [
  {
    key: 'prism-credit-free-negative',
    violations: sql`
      SELECT ${PrismCreditEntries.userId} AS id
      FROM ${PrismCreditEntries}
      GROUP BY ${PrismCreditEntries.userId}
      HAVING SUM(${PrismCreditEntries.freeDelta}) < 0
    `,
  },
  {
    key: 'prism-credit-refund-mismatch',
    violations: sql`
      SELECT r.id AS id
      FROM ${PrismCreditEntries} r
      LEFT JOIN ${PrismCreditEntries} c
        ON c.kind = 'REVIEW_CHARGE' AND c.key = r.key
      WHERE r.kind = 'REVIEW_REFUND'
        AND (c.id IS NULL OR c.paid_delta <> -r.paid_delta OR c.free_delta <> -r.free_delta OR c.user_id <> r.user_id)
    `,
  },
];

const runInvariantCheck = async (check: InvariantCheck) => {
  try {
    const rows = await dbr.execute<{ count: number; sample_ids: string[] | null }>(sql`
      WITH violations AS (${check.violations})
      SELECT
        COUNT(*)::int AS count,
        COALESCE(
          (SELECT array_agg(v.id) FROM (SELECT id FROM violations ORDER BY id LIMIT ${SAMPLE_LIMIT}) v),
          ARRAY[]::text[]
        ) AS sample_ids
      FROM violations
    `);

    const row = rows[0];
    if (row && row.count > 0) {
      await opsAlert('invariant-violation', { check: check.key, count: row.count, sampleIds: row.sample_ids ?? [] });
    }
  } catch (err) {
    Sentry.captureException(err, { extra: { check: check.key } });
  }
};

export const PrismCreditInvariantsCron = defineCron('prism:credit-invariants', '*/30 * * * *', async () => {
  for (const check of INVARIANT_CHECKS) {
    await runInvariantCheck(check);
  }
});

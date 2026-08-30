import '@typie/lib/dayjs';

import { parseArgs } from 'node:util';
import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
import { createDbId, db, TableCode } from '#/db/index.ts';
import { computeTrialExpiresAt, toMilli } from '#/utils/prism-credit-core.ts';

const BACKFILL_CREDIT_AMOUNT = 600;
const BACKFILL_NOTE = '기존 이용자 일괄 지급';
const BATCH_SIZE = 2000;

const { values } = parseArgs({ options: { yes: { type: 'boolean', default: false } } });
const dryRun = !values.yes;

const main = async () => {
  const expiresAt = computeTrialExpiresAt(dayjs());
  const freeDelta = toMilli(BACKFILL_CREDIT_AMOUNT);
  const { host, port, database } = db.$client.options;

  console.log(`프리즘 체험 크레딧 백필 시작 — ${dryRun ? 'DRY RUN(쓰기 없음)' : '실행'}`);
  console.log(`  대상 DB: ${host.join(',')}:${port.join(',')}/${database}`);
  console.log(`  지급량: ${BACKFILL_CREDIT_AMOUNT} 크레딧 / 만료: ${expiresAt.toISOString()}`);

  const targets = await db.execute<{ id: string }>(sql`
    SELECT u.id
    FROM users u
    WHERE NOT EXISTS (SELECT 1 FROM prism_credit_entries e WHERE e.kind = 'TRIAL' AND e.key = u.id)
    ORDER BY u.id
  `);

  console.log(`  대상: ${targets.length}건`);

  if (targets.length === 0) {
    console.log('완료 — 지급할 대상이 없다');

    return;
  }

  if (dryRun) {
    for (const target of targets.slice(0, 5)) {
      console.log(`    ${target.id}`);
    }
    console.log(`  … 외 ${Math.max(0, targets.length - 5)}건`);
    console.log('DRY RUN — 쓰기 없이 종료 (실제 지급은 --yes)');

    return;
  }

  let written = 0;
  for (let offset = 0; offset < targets.length; offset += BATCH_SIZE) {
    const batch = targets.slice(offset, offset + BATCH_SIZE);
    const values = sql.join(
      batch.map(
        (target) =>
          sql`(${createDbId(TableCode.PRISM_CREDIT_ENTRIES)}, ${target.id}, 'TRIAL', 0, ${freeDelta}, ${target.id}, ${BACKFILL_NOTE}, ${expiresAt.toISOString()}::timestamptz)`,
      ),
      sql`, `,
    );

    const inserted = await db.execute<{ id: string }>(sql`
      INSERT INTO prism_credit_entries (id, user_id, kind, paid_delta, free_delta, key, note, expires_at)
      VALUES ${values}
      ON CONFLICT (kind, key) DO NOTHING
      RETURNING id
    `);

    written += inserted.length;
    console.log(`  진행: ${Math.min(offset + BATCH_SIZE, targets.length)}/${targets.length}건 (기록 ${written})`);
  }

  const remaining = await db.execute<{ count: number }>(sql`
    SELECT count(*)::int AS count
    FROM users u
    WHERE NOT EXISTS (SELECT 1 FROM prism_credit_entries e WHERE e.kind = 'TRIAL' AND e.key = u.id)
  `);

  console.log(`완료 — 기록 ${written}건 / 잔여 ${remaining[0]?.count ?? '?'}건`);

  if (written < targets.length) {
    console.log(`  기록이 대상 ${targets.length}건보다 적은 것은 실행 중 가입한 계정이 이미 TRIAL을 받아`);
    console.log('  ON CONFLICT 로 건너뛴 결과다 — 멱등 처리가 동작한 것이지 실패가 아니다.');
  }

  if ((remaining[0]?.count ?? 0) > 0) {
    process.exitCode = 1;
  }
};

await main();
await db.$client.end();

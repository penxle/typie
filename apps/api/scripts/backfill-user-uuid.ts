import { sql } from 'drizzle-orm';
import * as uuid from 'uuid';
import { db, Users } from '#/db/index.ts';

// users.uuid 백필 — 기존 행만 구 파생식(uuid v5)으로 채운다.
//
// 파생은 설계 속성이 아니라 호환 요구다. 이미 발급된 스토어 트랜잭션의 appAccountToken·
// obfuscatedExternalAccountId 에 이 값이 박혀 있어, 기존 유저는 같은 값을 유지해야 그 트랜잭션의
// 소유자를 찾을 수 있다. 신규 행은 스키마의 $defaultFn 이 난수를 넣는다.
//
// 재실행 안전: uuid IS NULL 인 행만 대상이라 이미 채운 행은 건드리지 않는다.

const USER_UUID_NAMESPACE = '1d394eb5-c61c-4c49-944e-05c9f9435adf';
const BATCH_SIZE = 2000;

const main = async () => {
  const dryRun = process.argv.includes('--dry-run');
  console.log(`users.uuid 백필 시작 — ${dryRun ? 'DRY RUN(쓰기 없음)' : '실행'}`);

  const targets = await db.execute<{ id: string }>(sql`SELECT id FROM ${Users} WHERE uuid IS NULL ORDER BY id`);
  console.log(`  대상: ${targets.length}건`);

  if (targets.length === 0) {
    console.log('완료 — 채울 행이 없다');
    return;
  }

  const pairs = targets.map((row) => ({ id: row.id, uuid: uuid.v5(row.id, USER_UUID_NAMESPACE) }));

  const duplicates = pairs.length - new Set(pairs.map((pair) => pair.uuid)).size;
  if (duplicates > 0) {
    console.error(`파생값이 ${duplicates}건 충돌한다 — 유니크 인덱스를 걸 수 없다. 중단한다.`);
    process.exitCode = 1;

    return;
  }

  if (dryRun) {
    for (const pair of pairs.slice(0, 5)) {
      console.log(`    ${pair.id} → ${pair.uuid}`);
    }
    console.log(`  … 외 ${Math.max(0, pairs.length - 5)}건`);
    console.log('DRY RUN — 쓰기 없이 종료');

    return;
  }

  let written = 0;
  for (let offset = 0; offset < pairs.length; offset += BATCH_SIZE) {
    const batch = pairs.slice(offset, offset + BATCH_SIZE);
    const values = sql.join(
      batch.map((pair) => sql`(${pair.id}, ${pair.uuid}::uuid)`),
      sql`, `,
    );

    const updated = await db.execute<{ id: string }>(sql`
      UPDATE ${Users} AS u SET uuid = v.uuid
      FROM (VALUES ${values}) AS v(id, uuid)
      WHERE u.id = v.id AND u.uuid IS NULL
      RETURNING u.id
    `);

    written += updated.length;
    console.log(`  진행: ${Math.min(offset + BATCH_SIZE, pairs.length)}/${pairs.length}건 (기록 ${written})`);
  }

  const remaining = await db.execute<{ count: number }>(sql`SELECT count(*)::int AS count FROM ${Users} WHERE uuid IS NULL`);
  console.log(`완료 — 기록 ${written}건 / 잔여 NULL ${remaining[0]?.count ?? '?'}건`);

  if ((remaining[0]?.count ?? 0) > 0) {
    process.exitCode = 1;
  }
};

await main();
await db.$client.end();

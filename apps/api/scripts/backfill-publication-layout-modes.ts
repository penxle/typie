#!/usr/bin/env node

// publication_versions.layout_mode 백필. 컬럼 도입 전에 쓰인 판은 저장된 그래프에서 root 의 layout_mode 를
// 다시 읽어 채운다. 신규 판은 writeVersion 이 발행 시점에 채우므로 대상이 아니다.
//
// dry-run 이 기본이다(쓰기 없음). 적용은 --yes.
//
//   미리보기: SCRIPT=1 doppler run --config prod_local -- node scripts/backfill-publication-layout-modes.ts 2>&1 | tee backfill-publication-layout-modes-dry.log
//   실행:     SCRIPT=1 doppler run --config prod_local -- node scripts/backfill-publication-layout-modes.ts --yes 2>&1 | tee backfill-publication-layout-modes.log
//
// 재실행 안전: layout_mode IS NULL 인 행만 대상이고 UPDATE 도 같은 술어를 다시 걸어 이미 채운 행은 건드리지 않는다.
// 그래프 해석에 실패한 행은 NULL 로 남기고 종료 코드로 알린다 — 읽기 경로는 NULL 을 엔진 기본값(연속 600)으로 폴백한다.

import { parseArgs } from 'node:util';
import { and, asc, eq, isNull, sql } from 'drizzle-orm';
import { db, PublicationVersions } from '#/db/index.ts';
import { extractPlainDocLayoutMode } from '#/utils/entity.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import type { PlainDoc } from '@typie/editor-ffi/server';

process.env.SCRIPT = '1';

const { values } = parseArgs({ options: { yes: { type: 'boolean', default: false } } });
const dryRun = !values.yes;

const main = async () => {
  console.log(dryRun ? 'DRY RUN (실제 적용은 --yes)' : 'APPLY MODE');

  const targets = await db
    .select({ id: PublicationVersions.id, publicationId: PublicationVersions.publicationId, version: PublicationVersions.version })
    .from(PublicationVersions)
    .where(isNull(PublicationVersions.layoutMode))
    .orderBy(asc(PublicationVersions.id));

  console.log(`대상: ${targets.length}건`);

  const counts = { continuous: 0, paginated: 0 };
  let written = 0;
  let failed = 0;

  for (const target of targets) {
    const row = await db
      .select({ graph: PublicationVersions.graph })
      .from(PublicationVersions)
      .where(eq(PublicationVersions.id, target.id))
      .then((rows) => rows[0]);
    if (!row) continue;

    let layoutMode;
    try {
      const plain = (await wasm.to_plain(row.graph)) as PlainDoc;
      layoutMode = extractPlainDocLayoutMode(plain);
    } catch (err) {
      failed += 1;
      console.error(`  ${target.id} (${target.publicationId} v${target.version}): 그래프 해석 실패 —`, err);
      continue;
    }

    counts[layoutMode.type] += 1;

    if (dryRun) continue;

    await db
      .update(PublicationVersions)
      .set({ layoutMode })
      .where(and(eq(PublicationVersions.id, target.id), isNull(PublicationVersions.layoutMode)));
    written += 1;
  }

  console.log(`분포: continuous ${counts.continuous} / paginated ${counts.paginated} / 해석 실패 ${failed}`);

  if (!dryRun) {
    const remaining = await db
      .select({ count: sql<number>`count(*)::int` })
      .from(PublicationVersions)
      .where(isNull(PublicationVersions.layoutMode))
      .then((rows) => rows[0]?.count ?? 0);
    console.log(`완료 — 기록 ${written}건 / 잔여 NULL ${remaining}건`);
  }

  if (failed > 0) {
    process.exitCode = 1;
  }
};

await main();
await db.$client.end();

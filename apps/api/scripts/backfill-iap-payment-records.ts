#!/usr/bin/env node

// in_app_purchase_records 백필 — 전 바인딩(재조정 비활성 마커 포함) 대상 스토어 이력 재수집.
// Apple 은 죽은 바인딩도 이력이 남아 소급되고, Google 의 소멸 토큰(gone)은 스킵으로 관측된다(수용한 공백).
// dry-run 이 기본: 수집 결과만 출력하고 DB 에 쓰지 않는다. --yes 로 실제 적재(멱등 upsert — 재실행 안전).
//
//   미리보기: doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/backfill-iap-payment-records.ts 2>&1 | tee backfill-iap-payment-records.dry.log
//   실행:     doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/backfill-iap-payment-records.ts --yes 2>&1 | tee backfill-iap-payment-records.log
//
// 출력은 수동 검수가 본체라 반드시 파일로 남긴다(위 tee). 프로세스는 exitCode 만 세우고 자연 종료한다.

import { asc } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, pg, pgb, pgr, UserInAppPurchases } from '#/db/index.ts';
import { collectIapPaymentRecords, ingestIapPayments } from '#/utils/iap-ingest.ts';

const apply = process.argv.includes('--yes');

const main = async () => {
  const bindings = await db
    .select({
      id: UserInAppPurchases.id,
      userId: UserInAppPurchases.userId,
      store: UserInAppPurchases.store,
      identifier: UserInAppPurchases.identifier,
    })
    .from(UserInAppPurchases)
    // 재실행 로그가 행 물리 순서로 흔들리면 실행 간 diff 가 신호를 잃는다.
    .orderBy(asc(UserInAppPurchases.id));

  console.log(`IAP 결제 기록 백필 — ${apply ? '실행' : 'DRY RUN'} / 대상 바인딩 ${bindings.length}건`);

  const netByCurrency = new Map<string, number>();
  const seen = new Set<string>();
  let recordCount = 0;
  let ok = 0;
  let skipped = 0;
  let failed = 0;

  for (const binding of bindings) {
    try {
      if (apply) {
        const outcome = await ingestIapPayments({ bindingId: binding.id });

        if (outcome.kind === 'skipped') {
          skipped += 1;
          console.log(`  ${binding.id} (${binding.store}): 스킵(${outcome.reason})`);
        } else {
          ok += 1;
          recordCount += outcome.records;
          console.log(`  ${binding.id} (${binding.store}): ${outcome.records}건 적재`);
        }

        continue;
      }

      const collection = await collectIapPaymentRecords(binding);

      if (collection.kind === 'skipped') {
        skipped += 1;
        console.log(`  ${binding.id} (${binding.store}): 스킵(${collection.reason})`);
        continue;
      }

      ok += 1;

      if (collection.records.length === 0) {
        console.log(`  ${binding.id} (${binding.store}): 0건`);
      }

      for (const record of collection.records) {
        recordCount += 1;

        // Apple V2 이력은 고객의 앱 전체 이력을 돌려주므로 같은 고객의 바인딩 둘이 동일 transactionId 를 각자 수집한다
        // (appAccountToken 정밀 귀속은 중복을 만드는 게 아니라 그 줄들의 user 를 일치시킨다). 적재는
        // (store, identifier) 로 접히므로 합계는 첫 관측만 센다 — 출력 줄은 어느 바인딩에서 왔는지가 감사 정보라
        // 중복 표시를 달아 전건 남긴다.
        const key = `${binding.store}:${record.identifier}`;
        const duplicate = seen.has(key);

        if (!duplicate) {
          seen.add(key);
          const net = Number(record.amount) - Number(record.refundedAmount ?? 0);
          netByCurrency.set(record.currency, (netByCurrency.get(record.currency) ?? 0) + net);
        }

        console.log(
          `  ${binding.id} (${binding.store}) ${record.identifier} ${record.state} ${record.amount} ${record.currency}` +
            ` product=${record.productId ?? '-'} purchasedAt=${record.purchasedAt.toISOString()}` +
            `${record.refundedAmount === null ? '' : ` 환불=${record.refundedAmount}`} user=${record.userId}${duplicate ? ' (중복)' : ''}`,
        );
      }
    } catch (err) {
      failed += 1;
      console.log(`  ${binding.id} (${binding.store}): 실패 — ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  console.log(
    `\n완료 — ${apply ? '적재' : 'DRY RUN'} / 기록 ${recordCount}건${apply ? '' : `(고유 ${seen.size}건)`} / 스킵 ${skipped}건 / 실패 ${failed}건`,
  );

  // 모든 루프 경로가 ok·skipped·failed 중 정확히 하나를 올린다 — 합이 어긋나는 경우는 한 바인딩이
  // ok 계상 후 출력 루프에서 던져 failed 로도 계상된 때다(바인딩 중간 실패로 인한 초과 계상).
  const balanced = bindings.length === ok + skipped + failed;
  console.log(`  완료 게이트: 총 ${bindings.length} = 처리 ${ok} + 스킵 ${skipped} + 실패 ${failed} → ${balanced ? 'OK' : '불일치'}`);

  if (!balanced) {
    console.error('완료 게이트 불일치 — 어떤 바인딩도 무음으로 통과시키지 않는다.');
  }

  if (!apply && netByCurrency.size > 0) {
    // dry-run 순액 합계는 근사 검산용이다(Number 부동소수 — 확정 대사는 적재 후 SQL 로 한다).
    for (const [currency, net] of [...netByCurrency].toSorted(([a], [b]) => a.localeCompare(b))) {
      console.log(`  순액 합계(근사): ${net} ${currency}`);
    }
  }

  if (!balanced || failed > 0) {
    process.exitCode = 1;
  }
};

await main();
await pg.end();
await pgr.end();
await pgb.end();
// ops-alert 경유로 상주하는 redis 커넥션까지 닫아야 이벤트 루프가 비어 자연 종료한다.
redis.disconnect();

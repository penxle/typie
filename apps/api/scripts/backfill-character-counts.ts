#!/usr/bin/env node

// 글자 수 집계 단일화 이후 기존 문서의 text/characterCount 재계산.
//   미리보기: SCRIPT=1 NODE_ENV=production WASM_POOL_SIZE=1 DB_POOL_MAX=2 doppler run --config prod_local -- \
//     node scripts/backfill-character-counts.ts --dry-run 2>&1 | tee backfill-cc-dry.log
//   실행:     SCRIPT=1 NODE_ENV=production WASM_POOL_SIZE=1 DB_POOL_MAX=2 doppler run --config prod_local -- \
//     node scripts/backfill-character-counts.ts 2>&1 | tee backfill-cc.log
// 문서별 재계산은 멱등이라 중단 후 재실행해도 안전하다. 대부분의 실패 문서는 갱신되지 않아 다음 실행이 다시 집는다(예외는 아래 재색인 항목).
//
// 미리보기 로그는 progress:/done: 의 changed= 로 읽지 않는다 — dry-run 은 갱신 직전에 빠져나오므로 항상 0 이다.
// 변경될 문서 수는 grep -c '^\[dry\]' backfill-cc-dry.log 로 센다.
//
// 실패 목록에 오른 문서는 재색인 상태를 별도로 확인한다 — 커밋 직후 응답이 유실돼 실패로 잡힌 경우 값은 이미
// 갱신돼 있어 재실행이 동등 게이트(text/characterCount 일치)에 걸려 빠져나가고 enqueue 를 다시 하지 않는다.
//
// 주의: 기동만으로(미리보기 포함) 프로덕션 큐의 크론 스케줄러 7건이 이 체크아웃의 정의로 재등록된다
// — mq/index.ts:18-23 의 upsertJobScheduler 는 임포트 시점에 무조건 덮어쓴다. 반드시 배포본과 같은
// 리비전의 체크아웃에서 실행할 것.
//
// 실패 판정은 종료 코드가 아니라 done: 줄과 그 뒤의 실패 목록 출력으로 한다 — mq 의 Redis 소켓이
// 이벤트 루프를 잡고 있어 프로세스가 자연 종료하지 않는다. done: 을 확인한 뒤 Ctrl-C 로 끝낸다.
//
// SCRIPT=1 은 필수다 — #/mq/index.ts 를 임포트하면 mq/bullmq.ts 가 평가 시점에 worker.run() 을 돌려
// 이 프로세스가 프로덕션 잡을 집어삼킨다(bullmq.ts:72-73). ESM 은 임포트를 본문보다 먼저 평가하므로
// 파일 안에서 process.env.SCRIPT 를 세워도 늦다. 프로세스 시작 전에 주입해야 한다.
//
// NODE_ENV=production 도 필수다 — 잡 레인 이름이 dev 면 os.hostname(), 아니면 stack 이다(bullmq.ts:10,
// env.ts:55). 워크스테이션에서 NODE_ENV 없이 돌리면 재색인 잡이 아무 워커도 소비하지 않는 hostname 레인에
// 쌓여 조용히 유실된다.
//
// --config prod_local 은 doppler 기본 config(dev_local) 를 덮기 위한 것이다 — 이 디렉토리의 프로덕션
// 스크립트 관례(verify-sweep-readiness.ts, sweep-zombie-documents.ts, migrate-comment-anchors.ts).
// 실제 config 명칭은 실행 승인 시 오너가 확인한다.

import { asc, eq, gt, sql } from 'drizzle-orm';
import { db, DocumentStates, first } from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import type { PlainDoc } from '@typie/editor-ffi/server';

const BATCH = 100;
const dryRun = process.argv.includes('--dry-run');

let cursor = '';
let scanned = 0;
let changed = 0;
let skippedDegraded = 0;
const failures: string[] = [];
const reindexFailures: string[] = [];

for (;;) {
  const rows = await db
    .select({ documentId: DocumentStates.documentId })
    .from(DocumentStates)
    .where(gt(DocumentStates.documentId, cursor))
    .orderBy(asc(DocumentStates.documentId))
    .limit(BATCH);

  if (rows.length === 0) {
    break;
  }

  for (const { documentId } of rows) {
    cursor = documentId;
    scanned += 1;

    try {
      const result = await db.transaction(async (tx) => {
        // collect와 같은 락으로 fold 중간 상태 위에 덮어쓰는 경쟁을 차단한다.
        await tx.execute(sql`SELECT pg_advisory_xact_lock(hashtextextended(${documentId}, 0))`);

        const state = await tx
          .select({
            json: DocumentStates.json,
            text: DocumentStates.text,
            characterCount: DocumentStates.characterCount,
            projectionDegraded: DocumentStates.projectionDegraded,
          })
          .from(DocumentStates)
          .where(eq(DocumentStates.documentId, documentId))
          .then(first);

        if (!state) {
          return null;
        }
        if (state.projectionDegraded) {
          skippedDegraded += 1;
          return null;
        }

        const { text, characterCount } = await wasmFfi.use((host) => {
          const text = host.extract_text(state.json as PlainDoc);
          return { text, characterCount: host.count_characters(text) };
        });

        if (text === state.text && characterCount === state.characterCount) {
          return null;
        }
        if (dryRun) {
          console.log(`[dry] ${documentId}: ${state.characterCount} -> ${characterCount}`);
          return null;
        }

        await tx.update(DocumentStates).set({ text, characterCount }).where(eq(DocumentStates.documentId, documentId));
        return { from: state.characterCount, to: characterCount };
      });

      if (result) {
        changed += 1;
        // 커밋 뒤에 남긴다 — enqueue 가 유실돼도 이 줄만으로 재색인 대상을 복원할 수 있다.
        console.log(`[changed] ${documentId}: ${result.from} -> ${result.to}`);

        try {
          await enqueueJob('search:index:document', documentId);
        } catch (err) {
          reindexFailures.push(documentId);
          console.error(`[reindex-failed] ${documentId}:`, err);
        }
      }
    } catch (err) {
      // 문서 1건의 실패로 전체를 멈추지 않는다 — 갱신되지 않았으니 다음 실행이 이 문서를 다시 집는다.
      failures.push(documentId);
      console.error(`[failed] ${documentId}:`, err);
    }
  }

  console.log(
    `progress: scanned=${scanned} changed=${changed} skippedDegraded=${skippedDegraded} failed=${failures.length} cursor=${cursor}`,
  );
}

console.log(
  `done: scanned=${scanned} changed=${changed} skippedDegraded=${skippedDegraded} failed=${failures.length}${dryRun ? ' (dry-run)' : ''}`,
);

if (reindexFailures.length > 0) {
  console.error(`수동 재색인 필요 ${reindexFailures.length}건 — 값은 갱신됐으나 재색인 잡이 등록되지 않았다:`);
  console.error(reindexFailures.join(' '));
}

if (failures.length > 0) {
  console.error(`실패 ${failures.length}건 — 재실행하면 이 문서들만 다시 시도한다:`);
  console.error(failures.join(' '));
  process.exitCode = 1;
}

#!/usr/bin/env node

// 도달 불가한 푸시 토큰을 정리한다. FCM validate-only(dryRun)로 각 토큰을 검증만 하고
// 기기엔 아무것도 전송하지 않으므로 유저에게는 어떤 알림도 뜨지 않는다.
// 등록 해제(registration-token-not-registered)·형식 오류(invalid-registration-token) 토큰만 삭제한다.
//
// 일시적 오류(레이트리밋·5xx 등)는 판정으로 취급하지 않고 백오프 후 재시도하며,
// 재시도까지 소진해도 검증되지 않은 토큰은 삭제하지 않고 보고만 한다(다음 실행에서 수렴).
//
// 미리보기: DRY_RUN=1 doppler run -- node scripts/prune-push-notification-tokens.ts
// 실제 적용: doppler run -- node scripts/prune-push-notification-tokens.ts
// 삭제율 가드 조정: ABORT_RATIO=0.95 doppler run -- node scripts/prune-push-notification-tokens.ts

import { inArray } from 'drizzle-orm';
import { FirebaseMessagingError } from 'firebase-admin/messaging';
import { db, UserPushNotificationTokens } from '#/db/index.ts';
import { messaging } from '#/external/firebase.ts';

const dryRun = !!process.env.DRY_RUN;
const abortRatio = Number(process.env.ABORT_RATIO ?? 0.9);

const CHUNK_SIZE = 100;
const BATCH_DELAY_MS = 200;
const BASE_BACKOFF_MS = 2000;
const MAX_ATTEMPTS = 6;

const PERMANENT_SKIP_CODES = [
  'invalid-argument',
  'invalid-recipient',
  'mismatched-credential',
  'third-party-auth-error',
  'authentication-error',
];

const chunk = <T>(items: T[], size: number): T[][] => {
  const chunks: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    chunks.push(items.slice(i, i + size));
  }
  return chunks;
};

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms);
  });

const rows = await db.select({ token: UserPushNotificationTokens.token }).from(UserPushNotificationTokens);
const tokens = rows.map((row) => row.token);

console.log(`총 토큰: ${tokens.length}개 (dryRun=${dryRun}, abortRatio=${abortRatio})`);

const dead: string[] = [];
const malformed: string[] = [];
const permanentCounts = new Map<string, number>();

let pending = tokens;
let attempt = 0;

while (pending.length > 0 && attempt < MAX_ATTEMPTS) {
  attempt += 1;
  const transient: string[] = [];
  let processed = 0;

  for (const batch of chunk(pending, CHUNK_SIZE)) {
    const result = await messaging.sendEachForMulticast({ tokens: batch, data: { type: 'ping' } }, /* validateOnly */ true);

    for (const [index, response] of result.responses.entries()) {
      const token = batch[index];
      if (response.success) {
        continue;
      }

      const error = response.error;
      if (error instanceof FirebaseMessagingError && error.hasCode('registration-token-not-registered')) {
        dead.push(token);
      } else if (error instanceof FirebaseMessagingError && error.hasCode('invalid-registration-token')) {
        malformed.push(token);
      } else if (error instanceof FirebaseMessagingError && PERMANENT_SKIP_CODES.some((code) => error.hasCode(code))) {
        permanentCounts.set(error.code, (permanentCounts.get(error.code) ?? 0) + 1);
      } else {
        transient.push(token);
      }
    }

    processed += batch.length;
    console.log(
      `  attempt ${attempt} 진행: ${processed}/${pending.length} (삭제대상 누적 ${dead.length + malformed.length}, 재시도 ${transient.length})`,
    );

    await sleep(BATCH_DELAY_MS);
  }

  pending = transient;
  if (pending.length > 0 && attempt < MAX_ATTEMPTS) {
    const backoff = BASE_BACKOFF_MS * 2 ** (attempt - 1);
    console.log(`  일시적 오류 ${pending.length}개 — ${backoff}ms 후 재시도`);
    await sleep(backoff);
  }
}

const unverified = pending;
const toDelete = [...dead, ...malformed];

let permanentTotal = 0;
for (const count of permanentCounts.values()) {
  permanentTotal += count;
}

const alive = tokens.length - toDelete.length - permanentTotal - unverified.length;

console.log('');
console.log('=== 결과 ===');
console.log(`유효: ${alive}`);
console.log(`삭제 대상: ${toDelete.length} (등록해제 ${dead.length}, 형식오류 ${malformed.length})`);
if (permanentTotal > 0) {
  console.log(`영구 오류(삭제하지 않음) ${permanentTotal}:`);
  for (const [code, count] of permanentCounts) {
    console.log(`  ${code}: ${count}`);
  }
}
if (unverified.length > 0) {
  console.log(`미검증(일시적 오류 지속, 삭제하지 않음) ${unverified.length} — 나중에 다시 실행하면 수렴`);
}

const verified = alive + toDelete.length;
const ratio = verified > 0 ? toDelete.length / verified : 0;
if (ratio > abortRatio) {
  console.error(
    `삭제 대상 비율이 비정상적으로 높습니다: ${(ratio * 100).toFixed(1)}% (${toDelete.length}/${verified}). ` +
      `설정/자격증명 문제가 아닌지 확인 후 ABORT_RATIO 로 재조정하세요. 중단합니다.`,
  );
  process.exit(1);
}

if (dryRun) {
  console.log('DRY_RUN — 실제 삭제 없음');
  process.exit(0);
}

let deleted = 0;
for (const batch of chunk(toDelete, CHUNK_SIZE)) {
  await db.delete(UserPushNotificationTokens).where(inArray(UserPushNotificationTokens.token, batch));
  deleted += batch.length;
}

console.log(`완료: ${deleted}개 삭제`);
process.exit(0);

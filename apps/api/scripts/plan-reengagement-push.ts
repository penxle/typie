#!/usr/bin/env node

// 무료체험 종료 안내(광고) 푸시를 아래 조건을 모두 만족하는 이용자에게 일괄 발송한다.
//  - WILL_EXPIRE 트라이얼(FULL_ACCESS_TRIAL)이 KST 7/27(월) 중 종료
//  - KST 7/13 00:00(유료화) 이후 집필 이력 있음
//  - 트라이얼 외 살아있는 구독 없음(현재 비구독자 + 전환 예약 WILL_ACTIVATE 제외)
//  - 푸시 토큰 보유
//  - 마케팅 수신 동의 완료
//
// 실제로 전달된 유저만 Redis 에 기록해 중복 발송을 막는다. 실패(토큰 소멸·일시 오류)는 기록하지
// 않으므로 재실행하면 실패분만 자동으로 다시 시도한다(크래시 후 이어서 실행 가능).
//
// 대상 수 미리보기(발송 없음): DRY_RUN=1 doppler run -- node scripts/plan-reengagement-push.ts
// 알림 표시 확인(한 명에게만):  doppler run -- node scripts/plan-reengagement-push.ts <userId>
// 실제 발송:                    doppler run -- node scripts/plan-reengagement-push.ts

import { PlanId } from '@typie/lib/const';
import { SubscriptionState, UserState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, exists, gt, gte, inArray, lt, ne, notExists, or } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, DocumentCharacterCountChanges, Subscriptions, UserMarketingConsents, UserPushNotificationTokens, Users } from '#/db/index.ts';
import { sendPushNotification } from '#/external/firebase.ts';
import { delay } from '#/utils/promise.ts';

const PUSH_TITLE = '(광고) 타이피 무료 이용 기간이 내일 끝나요';
const PUSH_BODY = ['지금 구독하고 월 2,900원으로 쓰던 글을 계속 이어가세요!', '(수신거부: 설정 > 프로필 > 수신 동의 해제)'].join('\n');

const WRITING_SINCE = dayjs('2026-07-13T00:00:00+09:00');
const TRIAL_EXPIRES_FROM = dayjs('2026-07-27T00:00:00+09:00');
const TRIAL_EXPIRES_UNTIL = dayjs('2026-07-28T00:00:00+09:00');

const REDIS_KEY = 'script:plan-reengagement-push:trial-conversion:sent';
const BATCH_SIZE = 20;
const BATCH_INTERVAL_MS = 500;

const dryRun = !!process.env.DRY_RUN;
const testUserId = process.argv[2];

if (testUserId) {
  console.log(`테스트 모드: ${testUserId} 에게만 발송합니다.`);
  const ok = await sendPushNotification({ userId: testUserId, title: PUSH_TITLE, body: PUSH_BODY });
  console.log(ok ? '✓ 발송 완료' : '✗ 발송 실패(푸시 토큰 없음 또는 오류)');
  process.exit(ok ? 0 : 1);
}

console.log('대상 조회 중...');
const targets = await db
  .select({ id: Users.id })
  .from(Users)
  .where(
    and(
      eq(Users.state, UserState.ACTIVE),
      // WILL_EXPIRE 트라이얼이 KST 7/27(월) 중 종료
      exists(
        db
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(
            and(
              eq(Subscriptions.userId, Users.id),
              eq(Subscriptions.planId, PlanId.FULL_ACCESS_TRIAL),
              eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
              gte(Subscriptions.expiresAt, TRIAL_EXPIRES_FROM),
              lt(Subscriptions.expiresAt, TRIAL_EXPIRES_UNTIL),
            ),
          ),
      ),
      // 마케팅 수신 동의 완료
      exists(
        db
          .select({ id: UserMarketingConsents.id })
          .from(UserMarketingConsents)
          .where(and(eq(UserMarketingConsents.userId, Users.id), eq(UserMarketingConsents.consented, true))),
      ),
      // 푸시 토큰 보유
      exists(
        db
          .select({ id: UserPushNotificationTokens.id })
          .from(UserPushNotificationTokens)
          .where(eq(UserPushNotificationTokens.userId, Users.id)),
      ),
      // 트라이얼 외 살아있는 구독 없음 — 이미 유료 전환했거나 전환 예약(WILL_ACTIVATE)한 이용자 제외
      notExists(
        db
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(
            and(
              eq(Subscriptions.userId, Users.id),
              ne(Subscriptions.planId, PlanId.FULL_ACCESS_TRIAL),
              or(
                inArray(Subscriptions.state, [
                  SubscriptionState.ACTIVE,
                  SubscriptionState.WILL_ACTIVATE,
                  SubscriptionState.IN_GRACE_PERIOD,
                ]),
                and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.expiresAt, dayjs())),
              ),
            ),
          ),
      ),
      // KST 7/13 00:00(유료화) 이후 집필 이력 있음
      exists(
        db
          .select({ id: DocumentCharacterCountChanges.id })
          .from(DocumentCharacterCountChanges)
          .where(and(eq(DocumentCharacterCountChanges.userId, Users.id), gte(DocumentCharacterCountChanges.bucket, WRITING_SINCE))),
      ),
    ),
  );

const sent = new Set(await redis.smembers(REDIS_KEY));
const pending = targets.filter((target) => !sent.has(target.id));

console.log(`조건 충족 대상: ${targets.length}명`);
console.log(`이미 발송됨:    ${sent.size}명`);
console.log(`발송 대상:      ${pending.length}명\n`);

if (dryRun) {
  console.log('DRY_RUN — 실제 발송 없음');
  console.log(
    `샘플: ${
      pending
        .slice(0, 10)
        .map((target) => target.id)
        .join(', ') || '(없음)'
    }`,
  );
  process.exit(0);
}

if (pending.length === 0) {
  console.log('새로 발송할 대상이 없습니다.');
  process.exit(0);
}

let success = 0;
let failure = 0;
const startTime = Date.now();

for (let i = 0; i < pending.length; i += BATCH_SIZE) {
  const batch = pending.slice(i, i + BATCH_SIZE);
  const batchStart = Date.now();

  await Promise.all(
    batch.map(async (user) => {
      try {
        const ok = await sendPushNotification({ userId: user.id, title: PUSH_TITLE, body: PUSH_BODY });
        if (ok) {
          // 전달 성공만 기록 — 실패는 재실행에서 다시 시도한다.
          await redis.sadd(REDIS_KEY, user.id);
          success++;
        } else {
          failure++;
        }
      } catch (err) {
        failure++;
        console.error(`  ✗ ${user.id}: ${(err as Error).message}`);
      }
    }),
  );

  const processed = success + failure;
  const elapsed = (Date.now() - startTime) / 1000;
  const rate = processed / elapsed;
  const remaining = pending.length - processed;
  const eta = rate > 0 ? Math.round(remaining / rate) : 0;
  const pct = ((processed / pending.length) * 100).toFixed(1).padStart(5);

  console.log(
    `[${String(processed).padStart(String(pending.length).length)}/${pending.length}] ${pct}% | 성공 ${success} / 실패 ${failure} | ${rate.toFixed(1).padStart(4)}/s | ETA ${eta}s`,
  );

  const batchElapsed = Date.now() - batchStart;
  if (i + BATCH_SIZE < pending.length && batchElapsed < BATCH_INTERVAL_MS) {
    await delay(BATCH_INTERVAL_MS - batchElapsed);
  }
}

const totalElapsed = ((Date.now() - startTime) / 1000).toFixed(1);
console.log(`\n완료: 성공 ${success}건 / 실패 ${failure}건 / 총 ${totalElapsed}초`);

process.exit(0);

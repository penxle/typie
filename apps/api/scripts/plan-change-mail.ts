#!/usr/bin/env node

import { eq } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, Users } from '#/db/index.ts';
import { sendEmail } from '#/email/index.ts';
import { PlanChangeEmail } from '#/email/templates/index.ts';
import { delay } from '#/utils/promise.ts';

const REDIS_KEY = 'script:plan-change-mail:sent';
const BATCH_SIZE = 10;
const BATCH_INTERVAL_MS = 1000;

const testRecipient = process.argv[2];

if (testRecipient) {
  console.log(`테스트 모드: ${testRecipient}로 발송합니다.`);
  await sendEmail({
    subject: '[중요] 7월 1일, 타이피 플랜 구성 및 구독 요금이 변경됩니다',
    recipient: testRecipient,
    body: PlanChangeEmail(),
  });
  console.log(`✓ ${testRecipient}로 발송 완료`);
  process.exit(0);
}

console.log('ACTIVE 사용자 조회 중...');
const users = await db.select({ id: Users.id, email: Users.email }).from(Users).where(eq(Users.state, 'ACTIVE'));

const sent = new Set(await redis.smembers(REDIS_KEY));
const targets = users.filter((u) => !sent.has(u.id));

console.log(`전체 ACTIVE 사용자: ${users.length}명`);
console.log(`이미 발송됨:       ${sent.size}명`);
console.log(`발송 대상:         ${targets.length}명\n`);

if (targets.length === 0) {
  console.log('발송할 대상이 없습니다.');
  process.exit(0);
}

let success = 0;
let failure = 0;
const startTime = Date.now();

for (let i = 0; i < targets.length; i += BATCH_SIZE) {
  const batch = targets.slice(i, i + BATCH_SIZE);
  const batchStart = Date.now();

  await Promise.all(
    batch.map(async (user) => {
      try {
        await sendEmail({
          subject: '[중요] 7월 1일, 타이피 플랜 구성 및 구독 요금이 변경됩니다',
          recipient: user.email,
          body: PlanChangeEmail(),
        });
        await redis.sadd(REDIS_KEY, user.id);
        success++;
      } catch (err) {
        failure++;
        console.error(`  ✗ ${user.email}: ${(err as Error).message}`);
      }
    }),
  );

  const processed = success + failure;
  const elapsed = (Date.now() - startTime) / 1000;
  const rate = processed / elapsed;
  const remaining = targets.length - processed;
  const eta = rate > 0 ? Math.round(remaining / rate) : 0;
  const pct = ((processed / targets.length) * 100).toFixed(1).padStart(5);

  console.log(
    `[${String(processed).padStart(String(targets.length).length)}/${targets.length}] ${pct}% | 성공 ${success} / 실패 ${failure} | ${rate.toFixed(1).padStart(4)}/s | ETA ${eta}s`,
  );

  const batchElapsed = Date.now() - batchStart;
  if (i + BATCH_SIZE < targets.length && batchElapsed < BATCH_INTERVAL_MS) {
    await delay(BATCH_INTERVAL_MS - batchElapsed);
  }
}

const totalElapsed = ((Date.now() - startTime) / 1000).toFixed(1);
console.log(`\n완료: 성공 ${success}건 / 실패 ${failure}건 / 총 ${totalElapsed}초`);

process.exit(0);

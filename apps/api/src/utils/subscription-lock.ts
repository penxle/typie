import { sql } from 'drizzle-orm';
import type { Transaction } from '#/db/index.ts';

// 구독·빌링키 상태를 쓰는 트랜잭션의 유저 단위 직렬화. 다른 행 잠금보다 먼저 호출한다.
// lock_timeout 은 advisory 대기가 커넥션 풀을 무한 점유하는 것을 막는 상한이다(초과 시 실패·롤백,
// 같은 트랜잭션의 이후 행 잠금 대기에도 적용된다).
export const lockUserSubscriptionState = async (tx: Transaction, userId: string) => {
  await tx.execute(sql`SET LOCAL lock_timeout = '15s'`);
  await tx.execute(sql`SELECT pg_advisory_xact_lock(hashtextextended(${userId}, 0))`);
};

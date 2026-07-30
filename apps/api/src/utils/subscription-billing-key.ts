import * as Sentry from '@sentry/node';
import { PlanAvailability, PlanInterval, SubscriptionState, UserState } from '@typie/lib/enums';
import { and, eq, ne } from 'drizzle-orm';
import { db, first, firstOrThrow, Plans, Subscriptions, UserBillingKeys, Users } from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { lockUserSubscriptionState } from './subscription-lock.ts';
import type { BillingKeyType } from '@typie/lib/enums';
import type { Database, Transaction } from '#/db/index.ts';

export const hasLiveYearlyBillingKeySubscription = async (db: Database | Transaction, userId: string) => {
  const row = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.userId, userId),
        ne(Subscriptions.state, SubscriptionState.EXPIRED),
        eq(Plans.availability, PlanAvailability.BILLING_KEY),
        eq(Plans.interval, PlanInterval.YEARLY),
      ),
    )
    .then(first);

  return !!row;
};

type ReplaceUserBillingKeyParams = {
  userId: string;
  name: string;
  type: BillingKeyType;
  billingKey: string;
  guard?: (tx: Transaction) => Promise<void>;
};

export const replaceUserBillingKey = async (params: ReplaceUserBillingKeyParams) => {
  try {
    return await db.transaction(async (tx) => {
      await lockUserSubscriptionState(tx, params.userId);

      // 발급 대기 중 탈퇴가 완료됐으면 키를 재삽입하지 않는다.
      await tx
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.id, params.userId), eq(Users.state, UserState.ACTIVE)))
        .then(firstOrThrow);

      await params.guard?.(tx);

      const existingBillingKey = await tx
        .delete(UserBillingKeys)
        .where(eq(UserBillingKeys.userId, params.userId))
        .returning({ billingKey: UserBillingKeys.billingKey })
        .then(first);

      // 간편결제는 클라이언트가 키를 들고 오므로 같은 키가 다시 들어올 수 있다 — 회수하면 방금 등록한 키가 죽는다.
      if (existingBillingKey && existingBillingKey.billingKey !== params.billingKey) {
        await portone.deleteBillingKey({ billingKey: existingBillingKey.billingKey });
      }

      return await tx
        .insert(UserBillingKeys)
        .values({
          userId: params.userId,
          name: params.name,
          type: params.type,
          billingKey: params.billingKey,
        })
        .returning()
        .then(firstOrThrow);
    });
  } catch (err) {
    // 저장 실패(탈퇴 경합 등) 시 방금 발급한 외부 키가 로컬 참조 없는 고아로 남지 않게 회수한다.
    // deleteBillingKey 는 throw 하지 않고 상태를 반환하므로, 결과를 검사해야 회수 실패가 무음으로 사라지지 않는다.
    const deletion = await portone.deleteBillingKey({ billingKey: params.billingKey });
    if (deletion.status === 'failed') {
      Sentry.captureMessage('billing key compensation cleanup failed', {
        level: 'warning',
        extra: { userId: params.userId, billingKey: params.billingKey, code: deletion.code, message: deletion.message },
      });
    }
    throw err;
  }
};

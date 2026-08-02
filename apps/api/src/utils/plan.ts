import { PlanId } from '@typie/lib/const';
import { PaymentInvoiceState, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, eq, inArray, ne } from 'drizzle-orm';
import { db, first, firstOrThrow, PaymentInvoices, Plans, Subscriptions, UserTrials } from '#/db/index.ts';
import { resolveUserEntitlement } from './entitlement.ts';
import type { Transaction } from '#/db/index.ts';

export const ACTIVE_SUBSCRIPTION_STATES = [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD];

type AssertActiveSubscriptionParams = {
  userId: string;
};

export const hasActiveSubscription = async ({ userId }: { userId: string }) => {
  const rows = await db
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      planAvailability: Plans.availability,
      startsAt: Subscriptions.startsAt,
      currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
      currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
      createdAt: Subscriptions.createdAt,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(and(eq(Subscriptions.userId, userId), ne(Subscriptions.state, SubscriptionState.EXPIRED)));

  return resolveUserEntitlement(rows, dayjs()).entitled;
};

export const assertActiveSubscription = async ({ userId }: AssertActiveSubscriptionParams) => {
  if (!(await hasActiveSubscription({ userId }))) {
    throw new TypieError({ code: 'subscription_required', status: 403 });
  }
};

// 미래 청구 의무: 빌링키 채널의 살아있는 구독 또는 열린 인보이스. 빌링키 삭제·정리 경로가 공유한다 —
// 열린 OVERDUE 가 있는데 키를 지우면 유예 재시도 경로가 영구히 파괴된다.
export const hasFutureBillingObligation = async (tx: Transaction, userId: string) => {
  const subscription = await tx
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.userId, userId),
        inArray(Subscriptions.state, [
          SubscriptionState.ACTIVE,
          SubscriptionState.WILL_ACTIVATE,
          SubscriptionState.WILL_EXPIRE,
          SubscriptionState.IN_GRACE_PERIOD,
        ]),
        eq(Plans.availability, PlanAvailability.BILLING_KEY),
      ),
    )
    .then(first);

  if (subscription) {
    return true;
  }

  const invoice = await tx
    .select({ id: PaymentInvoices.id })
    .from(PaymentInvoices)
    .where(
      and(eq(PaymentInvoices.userId, userId), inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE])),
    )
    .then(first);

  return !!invoice;
};

type CreateTrialSubscriptionParams = {
  userId: string;
  startsAt: dayjs.Dayjs;
  expiresAt: dayjs.Dayjs;
};

export const createTrialSubscription = async (tx: Transaction, { userId, startsAt, expiresAt }: CreateTrialSubscriptionParams) => {
  const subscription = await tx
    .insert(Subscriptions)
    .values({
      userId,
      planId: PlanId.FULL_ACCESS_TRIAL,
      startsAt,
      currentPeriodStartsAt: startsAt,
      currentPeriodEndsAt: expiresAt,
      state: SubscriptionState.WILL_EXPIRE,
    })
    .returning()
    .then(firstOrThrow);

  await tx
    .insert(UserTrials)
    .values({ userId, subscriptionId: subscription.id, startedAt: startsAt, expiresAt })
    .onConflictDoUpdate({
      target: UserTrials.userId,
      set: { subscriptionId: subscription.id, startedAt: startsAt, expiresAt },
    });

  return subscription;
};

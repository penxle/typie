import {
  AccountTenure,
  ConsumptionStatus,
  DeliveryStatus,
  LifetimeDollarsPurchased,
  LifetimeDollarsRefunded,
  Platform,
  PlayTime,
  UserStatus,
} from '@apple/app-store-server-library';
import dayjs from 'dayjs';
import { and, eq, inArray, sql } from 'drizzle-orm';
import { Hono } from 'hono';
import { match } from 'ts-pattern';
import { db, first, PaymentInvoices, Plans, Subscriptions, UserInAppPurchases, Users, UserTrials } from '@/db';
import { InAppPurchaseStore, PaymentInvoiceState, PlanAvailability, SubscriptionState } from '@/enums';
import { production } from '@/env';
import * as appstore from '@/external/appstore';
import * as googleplay from '@/external/googleplay';
import * as slack from '@/external/slack';
import type { ResponseBodyV2 } from '@apple/app-store-server-library';
import type { Env } from '@/context';
import type { DeveloperNotification } from '@/external/googleplay';

export const iap = new Hono<Env>();

iap.post('/appstore', async (c) => {
  const body = await c.req.json<ResponseBodyV2>();
  if (!body.signedPayload) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const notification = await appstore.decodeNotification(body.signedPayload);
  const originalTransactionId = notification.data.transaction?.originalTransactionId;
  const planId = notification.data.transaction?.productId?.toUpperCase();

  if (!originalTransactionId) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const inAppPurchase = await db
    .select({
      userId: UserInAppPurchases.userId,
    })
    .from(UserInAppPurchases)
    .where(and(eq(UserInAppPurchases.identifier, originalTransactionId), eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE)))
    .then(first);

  if (!inAppPurchase) {
    return c.json({}, 200);
  }

  const subscription = await db
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      expiresAt: Subscriptions.expiresAt,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.userId, inAppPurchase.userId),
        eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
        inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
      ),
    )
    .then(first);

  await match(notification.notificationType)
    .with('DID_RENEW', 'SUBSCRIBED', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({
            state: SubscriptionState.ACTIVE,
            expiresAt: dayjs(notification.data.transaction?.expiresDate),
          })
          .where(eq(Subscriptions.id, subscription.id));
      } else if (planId) {
        const plan = await db.select({ id: Plans.id }).from(Plans).where(eq(Plans.id, planId)).then(first);
        if (plan) {
          await db.insert(Subscriptions).values({
            userId: inAppPurchase.userId,
            planId,
            startsAt: dayjs(notification.data.transaction?.purchaseDate),
            expiresAt: dayjs(notification.data.transaction?.expiresDate),
            state: SubscriptionState.ACTIVE,
          });
        }
      }
    })
    .with('EXPIRED', 'GRACE_PERIOD_EXPIRED', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.id, subscription.id));
      }
    })
    .with('DID_CHANGE_RENEWAL_PREF', async () => {
      if (subscription && planId) {
        const plan = await db.select({ id: Plans.id }).from(Plans).where(eq(Plans.id, planId)).then(first);
        if (plan) {
          await db
            .update(Subscriptions)
            .set({ planId, expiresAt: dayjs(notification.data.transaction?.expiresDate) })
            .where(eq(Subscriptions.id, subscription.id));
        }
      }
    })
    .with('DID_CHANGE_RENEWAL_STATUS', async () => {
      if (subscription) {
        if (notification.subtype === 'AUTO_RENEW_DISABLED') {
          await db.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, subscription.id));
        } else if (notification.subtype === 'AUTO_RENEW_ENABLED' && subscription.state === SubscriptionState.WILL_EXPIRE) {
          await db.update(Subscriptions).set({ state: SubscriptionState.ACTIVE }).where(eq(Subscriptions.id, subscription.id));
        }
      }
    })
    .with('RENEWAL_EXTENDED', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ expiresAt: dayjs(notification.data.transaction?.expiresDate) })
          .where(eq(Subscriptions.id, subscription.id));
      }
    })
    .with('REFUND', 'REVOKE', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.id, subscription.id));
      }
    })
    .with('CONSUMPTION_REQUEST', async () => {
      const transactionId = notification.data.transaction?.transactionId;
      if (!transactionId) {
        return;
      }

      const user = await db
        .select({ createdAt: Users.createdAt, state: Users.state })
        .from(Users)
        .where(eq(Users.id, inAppPurchase.userId))
        .then(first);

      const trial = await db.select({ id: UserTrials.id }).from(UserTrials).where(eq(UserTrials.userId, inAppPurchase.userId)).then(first);

      // 총 결제 금액 (KRW)
      const paidTotal = await db
        .select({ total: sql<number>`coalesce(sum(${PaymentInvoices.amount}), 0)` })
        .from(PaymentInvoices)
        .where(and(eq(PaymentInvoices.userId, inAppPurchase.userId), eq(PaymentInvoices.state, PaymentInvoiceState.PAID)))
        .then(first);

      const inRange = <T>(value: number, ranges: [number, T][], fallback: T): T =>
        ranges.find(([threshold]) => value < threshold)?.[1] ?? fallback;

      const accountDays = user ? dayjs().diff(dayjs(user.createdAt), 'day') : 0;
      const accountTenure = inRange(
        accountDays,
        [
          [3, AccountTenure.ZERO_TO_THREE_DAYS],
          [10, AccountTenure.THREE_DAYS_TO_TEN_DAYS],
          [30, AccountTenure.TEN_DAYS_TO_THIRTY_DAYS],
          [90, AccountTenure.THIRTY_DAYS_TO_NINETY_DAYS],
          [180, AccountTenure.NINETY_DAYS_TO_ONE_HUNDRED_EIGHTY_DAYS],
          [365, AccountTenure.ONE_HUNDRED_EIGHTY_DAYS_TO_THREE_HUNDRED_SIXTY_FIVE_DAYS],
        ],
        AccountTenure.GREATER_THAN_THREE_HUNDRED_SIXTY_FIVE_DAYS,
      );

      // KRW → USD 근사 변환 (1 USD ≈ 1,400 KRW)
      const lifetimeUsd = (paidTotal?.total ?? 0) / 1400;
      const lifetimeDollarsPurchased = inRange(
        lifetimeUsd,
        [
          [1, LifetimeDollarsPurchased.ZERO_DOLLARS],
          [50, LifetimeDollarsPurchased.ONE_CENT_TO_FORTY_NINE_DOLLARS_AND_NINETY_NINE_CENTS],
          [100, LifetimeDollarsPurchased.FIFTY_DOLLARS_TO_NINETY_NINE_DOLLARS_AND_NINETY_NINE_CENTS],
          [500, LifetimeDollarsPurchased.ONE_HUNDRED_DOLLARS_TO_FOUR_HUNDRED_NINETY_NINE_DOLLARS_AND_NINETY_NINE_CENTS],
          [1000, LifetimeDollarsPurchased.FIVE_HUNDRED_DOLLARS_TO_NINE_HUNDRED_NINETY_NINE_DOLLARS_AND_NINETY_NINE_CENTS],
          [2000, LifetimeDollarsPurchased.ONE_THOUSAND_DOLLARS_TO_ONE_THOUSAND_NINE_HUNDRED_NINETY_NINE_DOLLARS_AND_NINETY_NINE_CENTS],
        ],
        LifetimeDollarsPurchased.TWO_THOUSAND_DOLLARS_OR_GREATER,
      );

      const userStatus = user?.state === 'DEACTIVATED' ? UserStatus.TERMINATED : UserStatus.ACTIVE;

      await appstore.sendConsumptionData(transactionId, {
        customerConsented: true,
        consumptionStatus: ConsumptionStatus.FULLY_CONSUMED,
        platform: Platform.APPLE,
        sampleContentProvided: !!trial,
        deliveryStatus: DeliveryStatus.DELIVERED_AND_WORKING_PROPERLY,
        appAccountToken: notification.data.transaction?.appAccountToken,
        accountTenure,
        playTime: PlayTime.UNDECLARED,
        lifetimeDollarsRefunded: LifetimeDollarsRefunded.ZERO_DOLLARS,
        lifetimeDollarsPurchased,
        userStatus,
        refundPreference: 2, // PREFER_DECLINE
      });
    })
    .with('DID_FAIL_TO_RENEW', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set(
            notification.subtype === 'GRACE_PERIOD'
              ? { state: SubscriptionState.IN_GRACE_PERIOD }
              : { state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` },
          )
          .where(eq(Subscriptions.id, subscription.id));
      }
    })
    .otherwise(async () => {
      await slack.sendMessage({
        channel: 'iap',
        username: '인앱결제 알림',
        iconEmoji: ':credit_card:',
        message: `\`\`\`\n${JSON.stringify({ source: 'rest/appstore', notification }, null, 2)}\n\`\`\``,
      });
    });

  return c.json({}, 200);
});

iap.post('/googleplay', async (c) => {
  const notification = await c.req.json<DeveloperNotification>();

  if (notification.subscriptionNotification) {
    const googlePlaySubscription = await googleplay.getSubscription(notification.subscriptionNotification.purchaseToken);

    const item = googlePlaySubscription.lineItems?.[0];
    const planId = item?.offerDetails?.basePlanId?.toUpperCase();

    if (!item || !planId) {
      return c.json({ error: 'invalid_request' }, 400);
    }

    const inAppPurchase = await db
      .select({
        userId: UserInAppPurchases.userId,
      })
      .from(UserInAppPurchases)
      .where(
        and(
          eq(UserInAppPurchases.identifier, notification.subscriptionNotification.purchaseToken),
          eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY),
        ),
      )
      .then(first);

    // 구글 플레이는 발생한 알림이 환경 상관 없이 prod/dev 모두 발송됨
    if (!inAppPurchase) {
      if (production) {
        // prod 환경에서는 inAppPurchase 없을 시 오류 반환하고 pubsub에 재시도 맡김
        return c.json({ error: 'invalid_request' }, 400);
      } else {
        // dev 환경에서는 inAppPurchase 없어도 무시함
        return c.json({}, 200);
      }
    }

    const subscription = await db
      .select({
        id: Subscriptions.id,
        state: Subscriptions.state,
        expiresAt: Subscriptions.expiresAt,
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(
        and(
          eq(Subscriptions.userId, inAppPurchase.userId),
          eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
          inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
        ),
      )
      .then(first);

    await match(googlePlaySubscription.subscriptionState)
      .with('SUBSCRIPTION_STATE_ACTIVE', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({
              state: SubscriptionState.ACTIVE,
              expiresAt: dayjs(item.expiryTime),
            })
            .where(eq(Subscriptions.id, subscription.id));
        } else {
          await db.insert(Subscriptions).values({
            userId: inAppPurchase.userId,
            planId,
            startsAt: dayjs(googlePlaySubscription.startTime),
            expiresAt: dayjs(item.expiryTime),
            state: SubscriptionState.ACTIVE,
          });
        }
      })
      .with('SUBSCRIPTION_STATE_EXPIRED', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
            .where(eq(Subscriptions.id, subscription.id));
        }
      })
      .with('SUBSCRIPTION_STATE_CANCELED', async () => {
        if (subscription) {
          await db.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, subscription.id));
        }
      })
      .with('SUBSCRIPTION_STATE_IN_GRACE_PERIOD', async () => {
        if (subscription) {
          await db.update(Subscriptions).set({ state: SubscriptionState.IN_GRACE_PERIOD }).where(eq(Subscriptions.id, subscription.id));
        }
      })
      .with('SUBSCRIPTION_STATE_ON_HOLD', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
            .where(eq(Subscriptions.id, subscription.id));
        }
      })
      .with('SUBSCRIPTION_STATE_PAUSED', async () => {
        if (subscription) {
          await db.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, subscription.id));
        }
      })
      .with('SUBSCRIPTION_STATE_PENDING', 'SUBSCRIPTION_STATE_PENDING_PURCHASE_CANCELED', async () => {
        // 결제 대기 중 또는 대기 중 취소 — 구독 미생성 상태이므로 처리 불필요
      })
      .otherwise(async () => {
        await slack.sendMessage({
          channel: 'iap',
          username: '인앱결제 알림',
          iconEmoji: ':credit_card:',
          message: `\`\`\`\n${JSON.stringify({ source: 'rest/googleplay', subscription }, null, 2)}\n\`\`\``,
        });
      });
  } else if (notification.voidedPurchaseNotification && notification.voidedPurchaseNotification.productType === 1) {
    const inAppPurchase = await db
      .select({ userId: UserInAppPurchases.userId })
      .from(UserInAppPurchases)
      .where(
        and(
          eq(UserInAppPurchases.identifier, notification.voidedPurchaseNotification.purchaseToken),
          eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY),
        ),
      )
      .then(first);

    if (inAppPurchase) {
      const subscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
        .where(
          and(
            eq(Subscriptions.userId, inAppPurchase.userId),
            eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
            inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
          ),
        )
        .then(first);

      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.id, subscription.id));
      }
    }
  } else {
    await slack.sendMessage({
      channel: 'iap',
      username: '인앱결제 알림',
      iconEmoji: ':credit_card:',
      message: `\`\`\`\n${JSON.stringify({ source: 'rest/googleplay', notification }, null, 2)}\n\`\`\``,
    });
  }

  return c.json({}, 200);
});

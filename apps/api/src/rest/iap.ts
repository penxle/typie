import { DeliveryStatus, RefundPreference } from '@apple/app-store-server-library';
import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, inArray, sql } from 'drizzle-orm';
import { Hono } from 'hono';
import { match } from 'ts-pattern';
import { db, first, Plans, Subscriptions, UserInAppPurchases, UserTrials } from '#/db/index.ts';
import { production } from '#/env.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import * as slack from '#/external/slack.ts';
import type { ResponseBodyV2 } from '@apple/app-store-server-library';
import type { Env } from '#/context.ts';
import type { DeveloperNotification } from '#/external/googleplay.ts';

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
            renewedAt: dayjs(notification.data.transaction?.purchaseDate),
            expiresAt: dayjs(notification.data.transaction?.expiresDate),
          })
          .where(eq(Subscriptions.id, subscription.id));
      } else if (planId) {
        const plan = await db.select({ id: Plans.id }).from(Plans).where(eq(Plans.id, planId)).then(first);
        if (plan) {
          const startsAt = dayjs(notification.data.transaction?.purchaseDate);
          await db.insert(Subscriptions).values({
            userId: inAppPurchase.userId,
            planId,
            startsAt,
            expiresAt: dayjs(notification.data.transaction?.expiresDate),
            renewedAt: startsAt,
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

      const trial = await db.select({ id: UserTrials.id }).from(UserTrials).where(eq(UserTrials.userId, inAppPurchase.userId)).then(first);

      await appstore.sendConsumptionInformation(transactionId, {
        customerConsented: true,
        sampleContentProvided: !!trial,
        deliveryStatus: DeliveryStatus.DELIVERED,
        refundPreference: RefundPreference.DECLINE,
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
      }
      // dev 환경에서는 inAppPurchase 없어도 무시함
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

    await match(googlePlaySubscription.subscriptionState)
      .with('SUBSCRIPTION_STATE_ACTIVE', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({
              state: SubscriptionState.ACTIVE,
              renewedAt: subscription.expiresAt,
              expiresAt: dayjs(item.expiryTime),
            })
            .where(eq(Subscriptions.id, subscription.id));
        } else {
          const startsAt = dayjs(googlePlaySubscription.startTime);
          await db.insert(Subscriptions).values({
            userId: inAppPurchase.userId,
            planId,
            startsAt,
            expiresAt: dayjs(item.expiryTime),
            renewedAt: startsAt,
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

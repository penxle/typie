import dayjs from 'dayjs';
import { and, eq, inArray } from 'drizzle-orm';
import { Hono } from 'hono';
import { match } from 'ts-pattern';
import { db, first, firstOrThrow, Plans, Subscriptions, UserInAppPurchases } from '@/db';
import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@/enums';
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

  if (!originalTransactionId || !planId) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const { userId } = await db
    .select({
      userId: UserInAppPurchases.userId,
    })
    .from(UserInAppPurchases)
    .where(and(eq(UserInAppPurchases.identifier, originalTransactionId), eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE)))
    .then(firstOrThrow);

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
        eq(Subscriptions.userId, userId),
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
      } else {
        await db.insert(Subscriptions).values({
          userId,
          planId,
          startsAt: dayjs(notification.data.transaction?.purchaseDate),
          expiresAt: dayjs(notification.data.transaction?.expiresDate),
          state: SubscriptionState.ACTIVE,
        });
      }
    })
    .with('EXPIRED', 'GRACE_PERIOD_EXPIRED', async () => {
      if (subscription) {
        await db.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, subscription.id));
      }
    })
    .with('DID_CHANGE_RENEWAL_PREF', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ planId, expiresAt: dayjs(notification.data.transaction?.expiresDate) })
          .where(eq(Subscriptions.id, subscription.id));
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
    .with('DID_FAIL_TO_RENEW', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ state: notification.subtype === 'GRACE_PERIOD' ? SubscriptionState.IN_GRACE_PERIOD : SubscriptionState.EXPIRED })
          .where(eq(Subscriptions.id, subscription.id));
      }
    })
    .otherwise(async () => {
      await slack.sendMessage({ channel: 'iap', message: JSON.stringify({ source: 'rest/appstore', notification }, null, 2) });
    });

  return c.json({}, 200);
});

iap.post('/googleplay', async (c) => {
  const notification = await c.req.json<DeveloperNotification>();
  await slack.sendMessage({
    channel: 'iap',
    message: JSON.stringify({ source: 'rest/googleplay', notification }, null, 2),
  });

  if (notification.subscriptionNotification) {
    const subscription = await googleplay.getSubscription(notification.subscriptionNotification.purchaseToken);

    await slack.sendMessage({
      channel: 'iap',
      message: JSON.stringify({ source: 'rest/googleplay', subscription }, null, 2),
    });
  }

  return c.json({}, 200);
});

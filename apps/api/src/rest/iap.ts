/* eslint-disable @typescript-eslint/no-non-null-assertion */

import { NotificationTypeV2 } from '@apple/app-store-server-library';
import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { match } from 'ts-pattern';
import { db, first, firstOrEmptyObject, firstOrThrow, Plans, UserIAPSubscriptions, UserPlans } from '@/db';
import { InAppPurchaseStore, UserPlanBillingCycle, UserPlanState } from '@/enums';
import { production } from '@/env';
import * as appstore from '@/external/appstore';
import * as googleplay from '@/external/googleplay';
import * as slack from '@/external/slack';
import type { ResponseBodyV2 } from '@apple/app-store-server-library';
import type { Dayjs } from 'dayjs';
import type { Env } from '@/context';
import type { DeveloperNotification } from '@/external/googleplay';

export const iap = new Hono<Env>();

type GetUserPlanParams = {
  store: InAppPurchaseStore;
  subscriptionId: string;
};
const getUserPlan = async ({ store, subscriptionId }: GetUserPlanParams) =>
  await db
    .select({
      id: UserPlans.id,
    })
    .from(UserPlans)
    .innerJoin(
      UserIAPSubscriptions,
      and(eq(UserPlans.userId, UserIAPSubscriptions.userId), eq(UserPlans.billingMethod, UserIAPSubscriptions.store)),
    )
    .where(and(eq(UserIAPSubscriptions.store, store), eq(UserIAPSubscriptions.subscriptionId, subscriptionId)))
    .then(first);

type RenewSubscriptionParams = {
  store: InAppPurchaseStore;
  subscriptionId: string;
  planId: string;
  billingCycle: UserPlanBillingCycle;
  expiresAt: Dayjs;
};
const renewSubscription = async ({ store, subscriptionId, planId, billingCycle, expiresAt }: RenewSubscriptionParams) => {
  const { userId, userPlan } = await db
    .select({
      userId: UserIAPSubscriptions.userId,
      userPlan: {
        id: UserPlans.id,
        billingMethod: UserPlans.billingMethod,
      },
    })
    .from(UserIAPSubscriptions)
    .leftJoin(UserPlans, eq(UserPlans.userId, UserIAPSubscriptions.userId))
    .where(and(eq(UserIAPSubscriptions.store, store), eq(UserIAPSubscriptions.subscriptionId, subscriptionId)))
    .then(firstOrEmptyObject);

  if (userId) {
    if (userPlan) {
      if (userPlan.billingMethod === store) {
        await db.update(UserPlans).set({ state: UserPlanState.ACTIVE, expiresAt }).where(eq(UserPlans.id, userPlan.id));
      }
    } else {
      const plan = await db
        .select({
          id: Plans.id,
          fee: Plans.fee,
        })
        .from(Plans)
        .where(eq(Plans.id, planId))
        .then(firstOrThrow);

      await db.insert(UserPlans).values({
        userId,
        billingMethod: store,
        expiresAt,
        planId,
        billingCycle,
        fee: plan.fee,
      });
    }
  }
};

type ExpireSubscriptionParams = {
  store: InAppPurchaseStore;
  subscriptionId: string;
};
const expireSubscription = async ({ store, subscriptionId }: ExpireSubscriptionParams) => {
  const userPlan = await getUserPlan({ store, subscriptionId });

  if (userPlan) {
    await db.delete(UserPlans).where(eq(UserPlans.id, userPlan.id));
  }
};

iap.post('/appstore', async (c) => {
  const body = await c.req.json<ResponseBodyV2>();
  if (!body.signedPayload) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const notification = await appstore.decodeNotification({
    environment: production ? 'production' : 'sandbox',
    signedPayload: body.signedPayload,
  });

  await slack.sendMessage({ channel: 'iap', message: JSON.stringify({ source: 'rest/appstore', notification }, null, 2) });

  const subscriptionId = notification.data.transaction!.originalTransactionId!;

  await match(notification.notificationType)
    .with(NotificationTypeV2.SUBSCRIBED, NotificationTypeV2.DID_RENEW, NotificationTypeV2.DID_CHANGE_RENEWAL_PREF, async () => {
      const { billingCycle, planId } = appstore.getPlanInfoByProductId(notification.data.transaction!.productId);

      await renewSubscription({
        store: InAppPurchaseStore.APP_STORE,
        subscriptionId,
        expiresAt: dayjs(notification.data.transaction!.expiresDate),
        planId,
        billingCycle,
      });
    })
    .with(NotificationTypeV2.DID_FAIL_TO_RENEW, async () => {
      if (!notification.subtype) {
        // GRACE_PERIOD 아닌 경우
        await expireSubscription({
          store: InAppPurchaseStore.APP_STORE,
          subscriptionId,
        });
      }
    })
    .with(NotificationTypeV2.EXPIRED, NotificationTypeV2.GRACE_PERIOD_EXPIRED, NotificationTypeV2.REFUND, async () => {
      await expireSubscription({
        store: InAppPurchaseStore.APP_STORE,
        subscriptionId,
      });
    })
    .otherwise(() => null);

  return c.json({}, 200);
});

iap.post('/googleplay', async (c) => {
  const notification = await c.req.json<DeveloperNotification>();
  await slack.sendMessage({
    channel: 'iap',
    message: JSON.stringify({ source: 'rest/googleplay', notification }, null, 2),
  });

  if (notification.subscriptionNotification) {
    const subscription = await googleplay.getSubscription({
      purchaseToken: notification.subscriptionNotification.purchaseToken,
    });

    await slack.sendMessage({
      channel: 'iap',
      message: JSON.stringify({ source: 'rest/googleplay', subscription }, null, 2),
    });
  }

  return c.json({}, 200);
});

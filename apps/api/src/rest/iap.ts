import { Hono } from 'hono';
import { production } from '@/env';
import * as appstore from '@/external/appstore';
import * as googleplay from '@/external/googleplay';
import { logToSlack } from '@/utils/slack';
import type { ResponseBodyV2 } from '@apple/app-store-server-library';
import type { Env } from '@/context';
import type { DeveloperNotification } from '@/external/googleplay';

export const iap = new Hono<Env>();

iap.post('/appstore', async (c) => {
  const body = await c.req.json<ResponseBodyV2>();
  if (!body.signedPayload) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const notification = await appstore.decodeNotification({
    environment: production ? 'production' : 'sandbox',
    signedPayload: body.signedPayload,
  });

  logToSlack('iap', { source: '/iap/appstore', notification: JSON.stringify(notification, null, 2) });

  return c.json({}, 200);
});

iap.post('/googleplay', async (c) => {
  const body = await c.req.json<DeveloperNotification>();
  logToSlack('iap', { source: '/iap/googleplay', notification: JSON.stringify(body, null, 2) });

  if (body.subscriptionNotification) {
    const subscription = await googleplay.getSubscription({
      purchaseToken: body.subscriptionNotification.purchaseToken,
    });

    logToSlack('iap', { source: '/iap/googleplay', subscription: JSON.stringify(subscription, null, 2) });
  }

  return c.json({}, 200);
});

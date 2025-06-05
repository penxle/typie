import { Hono } from 'hono';
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

  await slack.sendMessage({ channel: 'iap', message: JSON.stringify({ source: 'rest/appstore', notification }, null, 2) });

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

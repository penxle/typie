import { eq } from 'drizzle-orm';
import { cert, initializeApp } from 'firebase-admin/app';
import { FirebaseMessagingError, getMessaging } from 'firebase-admin/messaging';
import { redis } from '#/cache.ts';
import { db, UserPushNotificationTokens } from '#/db/index.ts';
import { env } from '#/env.ts';

export const app = initializeApp({
  credential: cert(JSON.parse(env.GOOGLE_SERVICE_ACCOUNT)),
});

export const messaging = getMessaging(app);

const PUSH_TTL_SECONDS = 7 * 24 * 60 * 60;

type SendPushNotificationParams = { userId: string; title: string; body: string };
export type PushDelivery = 'sent' | 'no-tokens' | 'failed';

export const sendPushNotification = async ({ userId, title, body }: SendPushNotificationParams): Promise<PushDelivery> => {
  const tokens = await db
    .select({ token: UserPushNotificationTokens.token })
    .from(UserPushNotificationTokens)
    .where(eq(UserPushNotificationTokens.userId, userId));

  if (tokens.length === 0) return 'no-tokens';

  let success = false;

  for (const { token } of tokens) {
    try {
      await messaging.send({
        token,
        notification: {
          title,
          body,
        },
        data: {
          click_action: 'FLUTTER_NOTIFICATION_CLICK',
        },
        apns: {
          payload: {
            aps: {
              sound: 'default',
            },
          },
        },
        android: {
          notification: {
            defaultSound: true,
            defaultLightSettings: true,
            defaultVibrateTimings: true,
          },
        },
      });

      success = true;
    } catch (err) {
      if (err instanceof FirebaseMessagingError && err.hasCode('registration-token-not-registered')) {
        await db.delete(UserPushNotificationTokens).where(eq(UserPushNotificationTokens.token, token));
      }
    }
  }

  return success ? 'sent' : 'failed';
};

export const sendPushNotificationOnce = async ({
  key,
  userId,
  title,
  body,
}: SendPushNotificationParams & { key: string }): Promise<PushDelivery> => {
  if ((await redis.get(key)) !== null) return 'sent';

  const delivery = await sendPushNotification({ userId, title, body });
  if (delivery !== 'failed') await redis.set(key, '1', 'EX', PUSH_TTL_SECONDS, 'NX');

  return delivery;
};

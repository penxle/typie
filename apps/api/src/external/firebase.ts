import { eq } from 'drizzle-orm';
import { cert, initializeApp } from 'firebase-admin/app';
import { FirebaseMessagingError, getMessaging } from 'firebase-admin/messaging';
import { db, UserPushNotificationTokens } from '@/db';
import { env } from '@/env';

export const app = initializeApp({
  credential: cert(JSON.parse(env.FIREBASE_SERVICE_ACCOUNT)),
});

export const messaging = getMessaging(app);

type SendPushNotificationParams = { userId: string; title: string; body: string };
export const sendPushNotification = async ({ userId, title, body }: SendPushNotificationParams) => {
  const tokens = await db
    .select({ token: UserPushNotificationTokens.token })
    .from(UserPushNotificationTokens)
    .where(eq(UserPushNotificationTokens.userId, userId));

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

  return success;
};

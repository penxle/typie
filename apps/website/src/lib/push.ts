import { initializeApp } from 'firebase/app';
import { getMessaging, getToken, isSupported } from 'firebase/messaging';
import { env } from '$env/dynamic/public';
import type { FirebaseApp } from 'firebase/app';

let app: FirebaseApp | undefined;

const firebaseApp = () => {
  app ??= initializeApp({
    apiKey: env.PUBLIC_FIREBASE_API_KEY,
    authDomain: env.PUBLIC_FIREBASE_AUTH_DOMAIN,
    projectId: env.PUBLIC_FIREBASE_PROJECT_ID,
    messagingSenderId: env.PUBLIC_FIREBASE_MESSAGING_SENDER_ID,
    appId: env.PUBLIC_FIREBASE_APP_ID,
  });

  return app;
};

export const pushSupported = async (): Promise<boolean> => (await isSupported().catch(() => false)) && 'serviceWorker' in navigator;

export const pushPermission = (): NotificationPermission | null => {
  if (typeof Notification === 'undefined') {
    return null;
  }

  return Notification.permission;
};

export const acquirePushToken = async (): Promise<string | null> => {
  if (!(await pushSupported())) {
    return null;
  }

  try {
    const registration = await navigator.serviceWorker.register('/firebase-messaging-sw.js');
    const messaging = getMessaging(firebaseApp());

    return await getToken(messaging, {
      vapidKey: env.PUBLIC_FIREBASE_VAPID_KEY,
      serviceWorkerRegistration: registration,
    });
  } catch {
    return null;
  }
};

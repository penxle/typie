import { initializeApp } from 'firebase/app';
import { deleteToken, getMessaging, getToken, isSupported } from 'firebase/messaging';
import { env } from '$env/dynamic/public';
import { createBrowserPushManager } from '$lib/browser-push';
import { mearieClient } from '$lib/graphql';
import { graphql } from '$mearie';
import type { FirebaseApp } from 'firebase/app';
import type { BrowserPushManager } from '$lib/browser-push';

let app: FirebaseApp | undefined;
let browserPushManager: BrowserPushManager | undefined;
const LOGOUT_PUSH_CLEANUP_TIMEOUT_MS = 2000;
const browserPushStorage = {
  getItem: (key: string) => localStorage.getItem(key),
  setItem: (key: string, value: string) => localStorage.setItem(key, value),
};

const registerPushTokenMutation = graphql(`
  mutation Push_RegisterPushToken_Mutation($input: RegisterPushNotificationTokenInput!) {
    registerPushNotificationToken(input: $input)
  }
`);

const unregisterPushTokenMutation = graphql(`
  mutation Push_UnregisterPushToken_Mutation($input: UnregisterPushNotificationTokenInput!) {
    unregisterPushNotificationToken(input: $input)
  }
`);

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

const acquirePushToken = async (): Promise<string | null> => {
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

const deletePushToken = async (): Promise<boolean> => {
  if (!(await pushSupported())) return true;
  return deleteToken(getMessaging(firebaseApp()));
};

export const getBrowserPushManager = (): BrowserPushManager => {
  browserPushManager ??= createBrowserPushManager({
    acquireToken: acquirePushToken,
    deleteToken: deletePushToken,
    getPermission: pushPermission,
    isSupported: pushSupported,
    registerToken: async (token) => {
      await mearieClient.mutation(registerPushTokenMutation, { input: { token } });
    },
    requestPermission: () => Notification.requestPermission(),
    storage: browserPushStorage,
    unregisterToken: async (token) => {
      await mearieClient.mutation(unregisterPushTokenMutation, { input: { token } });
    },
  });
  return browserPushManager;
};

export const cleanupBrowserPushForLogout = async (): Promise<void> => {
  try {
    await Promise.race([
      getBrowserPushManager().cleanupForLogout(),
      new Promise<void>((resolve) => setTimeout(resolve, LOGOUT_PUSH_CLEANUP_TIMEOUT_MS)),
    ]);
  } catch {
    // Logout must continue even when browser push cleanup is unavailable.
  }
};

import { androidpublisher, auth } from '@googleapis/androidpublisher';
import { env } from '@/env';

const client = androidpublisher({
  version: 'v3',
  auth: new auth.GoogleAuth({
    credentials: JSON.parse(env.GOOGLE_SERVICE_ACCOUNT),
    scopes: ['https://www.googleapis.com/auth/androidpublisher'],
  }),
});

export const getSubscription = async (purchaseToken: string) => {
  // spell-checker:disable-next-line
  const response = await client.purchases.subscriptionsv2.get({
    packageName: env.GOOGLE_PLAY_PACKAGE_NAME,
    token: purchaseToken,
  });

  return response.data;
};

export type OneTimeProductNotificationType = 1 | 2; // 1: ONE_TIME_PRODUCT_PURCHASED, 2: ONE_TIME_PRODUCT_CANCELED
export type VoidedProductType = 1 | 2; // 1: PRODUCT_TYPE_SUBSCRIPTION, 2: PRODUCT_TYPE_ONE_TIME
export type RefundType = 1 | 2; // 1: REFUND_TYPE_FULL_REFUND, 2: REFUND_TYPE_QUANTITY_BASED_PARTIAL_REFUND
export type SubscriptionNotificationType =
  | 1 // SUBSCRIPTION_RECOVERED
  | 2 // SUBSCRIPTION_RENEWED
  | 3 // SUBSCRIPTION_CANCELED
  | 4 // SUBSCRIPTION_PURCHASED
  | 5 // SUBSCRIPTION_ON_HOLD
  | 6 // SUBSCRIPTION_IN_GRACE_PERIOD
  | 7 // SUBSCRIPTION_RESTARTED
  | 8 // SUBSCRIPTION_PRICE_CHANGE_CONFIRMED
  | 9 // SUBSCRIPTION_DEFERRED
  | 10 // SUBSCRIPTION_PAUSED
  | 11 // SUBSCRIPTION_PAUSE_SCHEDULE_CHANGED
  | 12 // SUBSCRIPTION_REVOKED
  | 13 // SUBSCRIPTION_EXPIRED
  | 20; // SUBSCRIPTION_PENDING_PURCHASE_CANCELED

export type TestNotification = {
  version: string;
};

export type OneTimeProductNotification = {
  version: string;
  notificationType: OneTimeProductNotificationType;
  purchaseToken: string;
  sku: string;
};

export type SubscriptionNotification = {
  version: string;
  notificationType: SubscriptionNotificationType;
  purchaseToken: string;
  subscriptionId: string;
};

export type VoidedPurchaseNotification = {
  purchaseToken: string;
  orderId: string;
  productType: VoidedProductType;
  refundType: RefundType;
};

export type DeveloperNotification = {
  version: string;
  packageName: string;
  eventTimeMillis: number;
  oneTimeProductNotification?: OneTimeProductNotification;
  subscriptionNotification?: SubscriptionNotification;
  voidedPurchaseNotification?: VoidedPurchaseNotification;
  testNotification?: TestNotification;
};

import { androidpublisher, auth } from '@googleapis/androidpublisher';
import { env } from '#/env.ts';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';

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

// 만료 60일 경과 등으로 스토어가 purchase token 제공을 영구 중단한 경우(404/410). 재시도해도 복구되지 않는다.
export const isPurchaseTokenGoneError = (error: unknown): boolean => {
  const err = error as { status?: unknown; response?: { status?: unknown } } | null | undefined;
  const status = err?.response?.status ?? err?.status;
  return status === 404 || status === 410;
};

export type GoogleSubscriptionResult =
  | { kind: 'ok'; purchase: androidpublisher_v3.Schema$SubscriptionPurchaseV2 }
  | { kind: 'gone' } // 410 — 토큰 영구 소멸, 재시도해도 복구되지 않는다
  | { kind: 'not-found' } // 404 — 설정 오류일 수 있어 gone과 동일 취급하지 않는다
  | { kind: 'error' };

export const getSubscriptionV2 = async (purchaseToken: string): Promise<GoogleSubscriptionResult> => {
  try {
    // spell-checker:disable-next-line
    const response = await client.purchases.subscriptionsv2.get({
      packageName: env.GOOGLE_PLAY_PACKAGE_NAME,
      token: purchaseToken,
    });

    return { kind: 'ok', purchase: response.data };
  } catch (err_) {
    const err = err_ as { status?: unknown; response?: { status?: unknown } } | null | undefined;
    const status = err?.response?.status ?? err?.status;

    if (status === 410) {
      return { kind: 'gone' };
    }
    if (status === 404) {
      return { kind: 'not-found' };
    }

    return { kind: 'error' };
  }
};

export type GoogleOrderResult = { kind: 'ok'; order: androidpublisher_v3.Schema$Order } | { kind: 'not-found' } | { kind: 'error' };

// 열거한 주문 체인의 개별 조회 — 존재하지 않는 접미사는 not-found 로 흡수한다
// (batchget 은 주문 ID 하나라도 없으면 전체 요청이 실패해 쓰지 않는다).
export const getOrder = async (orderId: string): Promise<GoogleOrderResult> => {
  try {
    const response = await client.orders.get({
      packageName: env.GOOGLE_PLAY_PACKAGE_NAME,
      orderId,
    });

    return { kind: 'ok', order: response.data };
  } catch (err_) {
    const err = err_ as { status?: unknown; response?: { status?: unknown } } | null | undefined;
    const status = err?.response?.status ?? err?.status;

    if (status === 404 || status === 410) {
      return { kind: 'not-found' };
    }

    return { kind: 'error' };
  }
};

export const acknowledgeSubscription = async ({
  productId,
  purchaseToken,
}: {
  productId: string;
  purchaseToken: string;
}): Promise<void> => {
  await client.purchases.subscriptions.acknowledge({
    packageName: env.GOOGLE_PLAY_PACKAGE_NAME,
    subscriptionId: productId,
    token: purchaseToken,
  });
};

export type OneTimeProductNotificationType = 1 | 2; // 1: ONE_TIME_PRODUCT_PURCHASED, 2: ONE_TIME_PRODUCT_CANCELED
export type VoidedProductType = 1 | 2; // 1: PRODUCT_TYPE_SUBSCRIPTION, 2: PRODUCT_TYPE_ONE_TIME
export type RefundType = 1 | 2; // 1: REFUND_TYPE_FULL_REFUND, 2: REFUND_TYPE_QUANTITY_BASED_PARTIAL_REFUND
// 알림은 타입 무관 전부 조회 트리거라 개별 매핑하지 않는다 — 공식 20종 중 일부만 허용하던 닫힌 union은
// 신규 타입 수신 시 그 알림만 조용히 드롭한다.
export type SubscriptionNotificationType = number;

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
  subscriptionId?: string; // 수신 방어 — 소비처 없음
};

export type VoidedPurchaseNotification = {
  purchaseToken: string;
  orderId: string;
  productType: VoidedProductType;
  refundType: RefundType;
};

// chargeback 검토 요청. 마감 필드는 payload에 없다 — 봉투의 eventTimeMillis + 24시간을 소비처가 계산한다.
export type PendingRefundReviewNotification = {
  version?: string;
  pendingRefundToken?: string;
  orderId?: string;
  refundReason?: number;
  obfuscatedAccountId?: string;
  obfuscatedProfileId?: string;
};

export type DeveloperNotification = {
  version: string;
  packageName: string;
  eventTimeMillis: string; // wire상 int64 문자열 — 소비처는 Number(...) + Number.isFinite 검증 후 사용
  oneTimeProductNotification?: OneTimeProductNotification;
  subscriptionNotification?: SubscriptionNotification;
  voidedPurchaseNotification?: VoidedPurchaseNotification;
  testNotification?: TestNotification;
  pendingRefundReviewNotification?: PendingRefundReviewNotification;
};

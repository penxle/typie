import { AppStoreServerAPIClient, Environment, SignedDataVerifier, Status } from '@apple/app-store-server-library';
import ky from 'ky';
import { env } from '#/env.ts';
import type { ConsumptionRequest } from '@apple/app-store-server-library';

const certificateUrls = [
  'https://www.apple.com/appleca/AppleIncRootCertificate.cer',
  'https://www.apple.com/certificateauthority/AppleRootCA-G2.cer',
  'https://www.apple.com/certificateauthority/AppleRootCA-G3.cer',
];

const certificates = await Promise.all(certificateUrls.map(async (url) => Buffer.from(await ky.get(url).arrayBuffer())));

const clients = {
  production: new AppStoreServerAPIClient(
    env.APPLE_IAP_PRIVATE_KEY,
    env.APPLE_IAP_KEY_ID,
    env.APPLE_IAP_ISSUER_ID,
    env.APPLE_APP_BUNDLE_ID,
    Environment.PRODUCTION,
  ),
  sandbox: new AppStoreServerAPIClient(
    env.APPLE_IAP_PRIVATE_KEY,
    env.APPLE_IAP_KEY_ID,
    env.APPLE_IAP_ISSUER_ID,
    env.APPLE_APP_BUNDLE_ID,
    Environment.SANDBOX,
  ),
};

const verifiers = {
  production: new SignedDataVerifier(certificates, true, Environment.PRODUCTION, env.APPLE_APP_BUNDLE_ID, env.APPLE_APP_APPLE_ID),
  sandbox: new SignedDataVerifier(certificates, true, Environment.SANDBOX, env.APPLE_APP_BUNDLE_ID, env.APPLE_APP_APPLE_ID),
};

const environments = ['production', 'sandbox'] as const;

export const getSubscription = async (transactionId: string) => {
  for (const environment of environments) {
    const client = clients[environment];
    const verifier = verifiers[environment];

    try {
      const subscription = await client.getAllSubscriptionStatuses(transactionId, [Status.ACTIVE]);
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const transaction = await verifier.verifyAndDecodeTransaction(subscription.data![0].lastTransactions![0].signedTransactionInfo!);

      return transaction;
    } catch {
      // pass
    }
  }

  throw new Error('Transaction not found');
};

export type ReconcileSubscriptionStatus =
  | { kind: 'active'; expiresDate: number | undefined; productId: string | undefined }
  | { kind: 'grace'; expiresDate: number | undefined }
  | { kind: 'suspended' }
  | { kind: 'expired' }
  | { kind: 'revoked' }
  | { kind: 'unknown' } // 조회는 됐으나 이 트랜잭션을 못 찾음 / 다루지 않는 상태 — 안전하게 건너뜀
  | { kind: 'error' }; // 모든 환경에서 조회 실패(네트워크·자격증명·서명검증 등) — 호출부에서 재시도/알림해야 함

// 재조정용: 스토어가 명시적으로 상태를 반환한 경우에만 active/inactive 를 판정하고,
// 조회 실패(일시적 오류 포함)·모호 상태(BILLING_RETRY 등)는 unknown 으로 남겨 만료 오판정을 막는다.
export const getSubscriptionStatus = async (transactionId: string): Promise<ReconcileSubscriptionStatus> => {
  let anyLookupSucceeded = false;

  for (const environment of environments) {
    const client = clients[environment];
    const verifier = verifiers[environment];

    // matched 이후의 예외는 조회 실패가 아니라 서명 검증 실패다. unknown(안전 스킵)으로 위장하면 재시도·알림 없이
    // 오판정이 무기한 방치되므로 error 로 구분한다.
    let matched = false;
    try {
      const response = await client.getAllSubscriptionStatuses(transactionId);
      anyLookupSucceeded = true;
      const transactionInfo = (response.data ?? [])
        .flatMap((group) => group.lastTransactions ?? [])
        .find((item) => item.originalTransactionId === transactionId);
      if (!transactionInfo) {
        continue;
      }
      matched = true;

      if (transactionInfo.status === Status.ACTIVE) {
        let expiresDate: number | undefined;
        let productId: string | undefined;
        if (transactionInfo.signedTransactionInfo) {
          const transaction = await verifier.verifyAndDecodeTransaction(transactionInfo.signedTransactionInfo);
          expiresDate = transaction.expiresDate;
          productId = transaction.productId;
        }
        return { kind: 'active', expiresDate, productId };
      }

      if (transactionInfo.status === Status.BILLING_GRACE_PERIOD) {
        // 유예 기간의 실제 마감일은 트랜잭션 만료일(이미 과거)이 아니라 renewalInfo 에 있다.
        let expiresDate: number | undefined;
        if (transactionInfo.signedRenewalInfo) {
          const renewalInfo = await verifier.verifyAndDecodeRenewalInfo(transactionInfo.signedRenewalInfo);
          expiresDate = renewalInfo.gracePeriodExpiresDate;
        }
        return { kind: 'grace', expiresDate };
      }

      // BILLING_RETRY 는 재청구 중(권한 없음)이지만 복구 가능하므로 EXPIRED(종료)와 구분한다.
      if (transactionInfo.status === Status.BILLING_RETRY) {
        return { kind: 'suspended' };
      }

      if (transactionInfo.status === Status.EXPIRED) {
        return { kind: 'expired' };
      }

      if (transactionInfo.status === Status.REVOKED) {
        return { kind: 'revoked' };
      }

      return { kind: 'unknown' };
    } catch {
      if (matched) {
        return { kind: 'error' };
      }
      // 이 환경 조회 실패 — 다음 환경 시도
    }
  }

  // 조회에 한 번이라도 성공했으나 트랜잭션을 못 찾음 → unknown(안전 스킵). 전 환경 실패 → error(재시도/알림).
  return anyLookupSucceeded ? { kind: 'unknown' } : { kind: 'error' };
};

export const decodeNotification = async (signedPayload: string) => {
  for (const environment of environments) {
    const verifier = verifiers[environment];

    try {
      const notification = await verifier.verifyAndDecodeNotification(signedPayload);

      return {
        ...notification,
        data: {
          ...notification.data,
          transaction: notification.data?.signedTransactionInfo
            ? await verifier.verifyAndDecodeTransaction(notification.data.signedTransactionInfo)
            : undefined,
          signedRenewalInfo: undefined,
          signedTransactionInfo: undefined,
        },
      };
    } catch {
      // pass
    }
  }

  throw new Error('Notification verification failed');
};

export const sendConsumptionInformation = async (transactionId: string, consumptionRequest: ConsumptionRequest) => {
  for (const environment of environments) {
    const client = clients[environment];

    try {
      await client.sendConsumptionInformation(transactionId, consumptionRequest);
      return;
    } catch {
      // pass
    }
  }

  throw new Error('Failed to send consumption data');
};

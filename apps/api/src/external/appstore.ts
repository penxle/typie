import { AppStoreServerAPIClient, Environment, SignedDataVerifier, Status } from '@apple/app-store-server-library';
import ky from 'ky';
import { env } from '@/env';

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

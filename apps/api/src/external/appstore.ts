import { AppStoreServerAPIClient, Environment, SignedDataVerifier, Status } from '@apple/app-store-server-library';
import ky from 'ky';
import { match } from 'ts-pattern';
import { PlanId } from '@/const';
import { UserPlanBillingCycle } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';

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

type GetTransactionParams = { environment: 'production' | 'sandbox'; transactionId: string };
export const getTransaction = async ({ environment, transactionId }: GetTransactionParams) => {
  const client = clients[environment];
  const verifier = verifiers[environment];

  const transactionInfo = await client.getTransactionInfo(transactionId);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const transaction = await verifier.verifyAndDecodeTransaction(transactionInfo.signedTransactionInfo!);

  return transaction;
};

type GetSubscriptionParams = { environment: 'production' | 'sandbox'; transactionId: string };
export const getSubscription = async ({ environment, transactionId }: GetSubscriptionParams) => {
  const client = clients[environment];
  const verifier = verifiers[environment];

  const subscription = await client.getAllSubscriptionStatuses(transactionId, [Status.ACTIVE]);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const transaction = await verifier.verifyAndDecodeTransaction(subscription.data![0].lastTransactions![0].signedTransactionInfo!);

  return transaction;
};

type DecodeNotificationParams = { environment: 'production' | 'sandbox'; signedPayload: string };
export const decodeNotification = async ({ environment, signedPayload }: DecodeNotificationParams) => {
  const verifier = verifiers[environment];

  const notification = await verifier.verifyAndDecodeNotification(signedPayload);

  return {
    ...notification,
    data: {
      ...notification.data,
      transaction: notification.data?.signedTransactionInfo
        ? await verifier.verifyAndDecodeTransaction(notification.data.signedTransactionInfo)
        : undefined,
    },
  };
};

export const getPlanInfoByProductId = (productId: string | undefined) =>
  match(productId)
    .with('plan.full.1month', () => ({ planId: PlanId.PLUS, billingCycle: UserPlanBillingCycle.MONTHLY }))
    .with('plan.full.1year', () => ({ planId: PlanId.PLUS, billingCycle: UserPlanBillingCycle.YEARLY }))
    .otherwise(() => {
      throw new TypieError({ code: 'not_found' });
    });

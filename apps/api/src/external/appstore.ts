import { AppStoreServerAPIClient, Environment, SignedDataVerifier } from '@apple/app-store-server-library';
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

type GetTransactionParams = { environment: 'production' | 'sandbox'; transactionId: string };
export const getTransaction = async ({ environment, transactionId }: GetTransactionParams) => {
  const client = clients[environment];
  const verifier = verifiers[environment];

  const transactionInfo = await client.getTransactionInfo(transactionId);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const transaction = await verifier.verifyAndDecodeTransaction(transactionInfo.signedTransactionInfo!);

  return transaction;
};

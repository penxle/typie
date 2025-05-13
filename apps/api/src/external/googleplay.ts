import { androidpublisher, auth } from '@googleapis/androidpublisher';
import { env } from '@/env';

const client = androidpublisher({
  version: 'v3',
  auth: new auth.GoogleAuth({
    credentials: JSON.parse(env.GOOGLE_SERVICE_ACCOUNT),
    scopes: ['https://www.googleapis.com/auth/androidpublisher'],
  }),
});

type GetSubscriptionParams = { purchaseToken: string };
export const getSubscription = async ({ purchaseToken }: GetSubscriptionParams) => {
  // spell-checker:disable-next-line
  const response = await client.purchases.subscriptionsv2.get({
    packageName: env.GOOGLE_PLAY_PACKAGE_NAME,
    token: purchaseToken,
  });

  return response.data;
};

import { PortOneClient, RestError } from '@portone/server-sdk';
import { env } from '../env';

export const client = PortOneClient({ secret: env.PORTONE_API_SECRET });

type PortOneSuccessResult<T> = { status: 'succeeded' } & T;
type PortOneFailureResult = { status: 'failed'; code: string; message: string };

type PortOneResult<T> = PortOneSuccessResult<T> | PortOneFailureResult;

type IssueBillingKeyParams = {
  customerId: string;
  cardNumber: string;
  expiryYear: string;
  expiryMonth: string;
  birthOrBusinessRegistrationNumber: string;
  passwordTwoDigits: string;
};

type IssueBillingKeyResult = PortOneResult<{
  billingKey: string;
  cardName: string;
}>;
export const issueBillingKey = async (params: IssueBillingKeyParams): Promise<IssueBillingKeyResult> => {
  try {
    const {
      billingKeyInfo: { billingKey },
    } = await client.payment.billingKey.issueBillingKey({
      channelKey: env.PORTONE_CHANNEL_KEY,
      method: {
        card: {
          credential: {
            number: params.cardNumber,
            expiryYear: params.expiryYear,
            expiryMonth: params.expiryMonth,
            birthOrBusinessRegistrationNumber: params.birthOrBusinessRegistrationNumber,
            passwordTwoDigits: params.passwordTwoDigits,
          },
        },
      },
      customer: {
        id: params.customerId,
      },
    });

    const resp = await client.payment.billingKey.getBillingKeyInfo({ billingKey });

    if (!resp || resp.status !== 'ISSUED' || resp.methods?.[0].type !== 'BillingKeyPaymentMethodCard') {
      throw new Error('Failed to issue billing key');
    }

    /* eslint-disable @typescript-eslint/no-non-null-assertion */
    const card = resp.methods[0].card!;

    return makeSuccessResult({
      billingKey,
      cardName: card.name!,
    });
    /* eslint-enable @typescript-eslint/no-non-null-assertion */
  } catch (err) {
    return makeFailureResult(err);
  }
};

type DeleteBillingKeyParams = {
  billingKey: string;
};
type DeleteBillingKeyResult = PortOneResult<unknown>;
export const deleteBillingKey = async (params: DeleteBillingKeyParams): Promise<DeleteBillingKeyResult> => {
  try {
    await client.payment.billingKey.deleteBillingKey({ billingKey: params.billingKey });

    return makeSuccessResult({});
  } catch (err) {
    return makeFailureResult(err);
  }
};

type MakePaymentParams = {
  paymentId: string;
  billingKey: string;
  customerName: string;
  customerEmail: string;
  orderName: string;
  amount: number;
};
type MakePaymentResult = PortOneResult<{ approvalNumber: string; receiptUrl: string }>;
export const makePayment = async (params: MakePaymentParams): Promise<MakePaymentResult> => {
  try {
    await client.payment.payWithBillingKey({
      paymentId: params.paymentId,
      billingKey: params.billingKey,
      orderName: params.orderName,
      amount: { total: params.amount },
      currency: 'KRW',
      customer: {
        name: { full: params.customerName },
        email: params.customerEmail,
      },
    });

    const resp = await client.payment.getPayment({ paymentId: params.paymentId });

    if (!resp || resp.status !== 'PAID' || resp.method?.type !== 'PaymentMethodCard') {
      throw new Error('Failed to make payment');
    }

    /* eslint-disable @typescript-eslint/no-non-null-assertion */
    return makeSuccessResult({
      approvalNumber: resp.method.approvalNumber!,
      receiptUrl: resp.receiptUrl!,
    });
    /* eslint-enable @typescript-eslint/no-non-null-assertion */
  } catch (err) {
    return makeFailureResult(err);
  }
};

type GetPaymentParams = {
  paymentId: string;
};
type GetPaymentResult = PortOneResult<{ amount: { total: number }; customData: string | undefined }>;
export const getPayment = async (params: GetPaymentParams): Promise<GetPaymentResult> => {
  const resp = await client.payment.getPayment({
    paymentId: params.paymentId,
  });

  if (resp.status === 'PAID') {
    return makeSuccessResult(resp);
  }

  return makeFailureResult(resp);
};

const makeSuccessResult = <T>(data: T): PortOneSuccessResult<T> => {
  return { ...data, status: 'succeeded' };
};

const makeFailureResult = (error: unknown): PortOneFailureResult => {
  return {
    status: 'failed',
    ...(() => {
      if (error instanceof RestError && error.data.type === 'PG_PROVIDER') {
        // Narrowing PgProviderError https://portone-io.github.io/server-sdk/js/types/Common.PgProviderError.html
        return {
          code: error.data.pgCode,
          message: error.data.pgMessage,
        };
      } else if (error instanceof Error) {
        return {
          code: error.name,
          message: error.message,
        };
      } else {
        return {
          code: 'unknown',
          message: String(error),
        };
      }
    })(),
  };
};

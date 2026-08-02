import { PortOneClient, RestError } from '@portone/server-sdk';
import { env } from '../env.ts';
import type { BillingKeyIssuance } from '#/utils/billing-key.ts';

export const client = PortOneClient({ secret: env.PORTONE_API_SECRET });

type PortOneSuccessResult<T> = T & { status: 'succeeded' };
type PortOneFailureResult = { status: 'failed'; code: string; message: string; portoneErrorType: string | null };

type PortOneResult<T> = PortOneSuccessResult<T> | PortOneFailureResult;

type IssueBillingKeyParams = {
  customerId: string;
  cardNumber: string;
  expiryYear: string;
  expiryMonth: string;
  birthOrBusinessRegistrationNumber: string;
  passwordTwoDigits: string;
};
type IssueBillingKeyResult = PortOneResult<{ billingKey: string; cardName: string }>;
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

type DeleteBillingKeyParams = { billingKey: string };
type DeleteBillingKeyResult = PortOneResult<unknown>;
export const deleteBillingKey = async (params: DeleteBillingKeyParams): Promise<DeleteBillingKeyResult> => {
  try {
    await client.payment.billingKey.deleteBillingKey({ billingKey: params.billingKey });

    return makeSuccessResult({});
  } catch (err) {
    return makeFailureResult(err);
  }
};

type GetBillingKeyInfoParams = { billingKey: string };
type GetBillingKeyInfoResult = PortOneResult<{ issuance: BillingKeyIssuance }>;
export const getBillingKeyInfo = async (params: GetBillingKeyInfoParams): Promise<GetBillingKeyInfoResult> => {
  try {
    const resp = await client.payment.billingKey.getBillingKeyInfo({ billingKey: params.billingKey });

    return makeSuccessResult({
      issuance: {
        status: String(resp.status),
        customerId: resp.status === 'ISSUED' ? resp.customer.id : undefined,
        channelKeys: resp.status === 'ISSUED' ? resp.channels.map((channel) => channel.key).filter((key) => key !== undefined) : [],
      },
    });
  } catch (err) {
    return makeFailureResult(err);
  }
};

type PayWithBillingKeyParams = {
  paymentId: string;
  billingKey: string;
  customerName: string;
  customerEmail: string;
  orderName: string;
  amount: number;
};
export type PayWithBillingKeyResult = PortOneFailureResult | { status: 'succeeded'; pgTxId: string | null; paidAt: string | null };
export const payWithBillingKey = async (params: PayWithBillingKeyParams): Promise<PayWithBillingKeyResult> => {
  try {
    const response = await client.payment.payWithBillingKey({
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

    return { status: 'succeeded', pgTxId: response?.payment?.pgTxId ?? null, paidAt: response?.payment?.paidAt ?? null };
  } catch (err) {
    return makeFailureResult(err);
  }
};

type CancelPaymentParams = { paymentId: string; reason: string };
type CancelPaymentResult = PortOneResult<unknown>;
export const cancelPayment = async (params: CancelPaymentParams): Promise<CancelPaymentResult> => {
  try {
    await client.payment.cancelPayment({
      paymentId: params.paymentId,
      reason: params.reason,
    });

    return makeSuccessResult({});
  } catch (err) {
    return makeFailureResult(err);
  }
};

type GetPaymentParams = { paymentId: string };
type GetPaymentResult = PortOneResult<{ amount: number; customData: string | undefined }>;
export const getPayment = async (params: GetPaymentParams): Promise<GetPaymentResult> => {
  const resp = await client.payment.getPayment({
    paymentId: params.paymentId,
  });

  if (resp.status === 'PAID') {
    return makeSuccessResult({
      amount: resp.amount.total,
      customData: resp.customData,
    });
  }

  return makeFailureResult(resp);
};

export type LookupPaymentResult = { kind: 'paid'; amount: number } | { kind: 'not-paid'; paymentStatus: string } | { kind: 'error' };
export const lookupPayment = async (params: { paymentId: string }): Promise<LookupPaymentResult> => {
  try {
    const resp = await client.payment.getPayment({ paymentId: params.paymentId });

    if (resp.status === 'PAID') {
      return { kind: 'paid', amount: resp.amount.total };
    }

    return { kind: 'not-paid', paymentStatus: String(resp.status) };
  } catch {
    return { kind: 'error' };
  }
};

export type PaymentReceipt = { approvalNumber: string | null; receiptUrl: string | null };
export const getPaymentReceipt = async (params: { paymentId: string }): Promise<PaymentReceipt | null> => {
  try {
    const resp = await client.payment.getPayment({ paymentId: params.paymentId });

    if (resp.status !== 'PAID') {
      return null;
    }

    return {
      approvalNumber: resp.method?.type === 'PaymentMethodCard' ? (resp.method.approvalNumber ?? null) : null,
      receiptUrl: resp.receiptUrl ?? null,
    };
  } catch {
    return null;
  }
};

type GetIdentityVerificationParams = { identityVerificationId: string };
type GetIdentityVerificationResult = PortOneResult<{
  name: string;
  birthDate: string;
  gender: string;
  operator: string;
  phoneNumber: string;
  ci: string;
}>;
export const getIdentityVerification = async (params: GetIdentityVerificationParams): Promise<GetIdentityVerificationResult> => {
  const resp = await client.identityVerification.getIdentityVerification({
    identityVerificationId: params.identityVerificationId,
  });

  if (resp.status === 'VERIFIED') {
    /* eslint-disable @typescript-eslint/no-non-null-assertion */
    return makeSuccessResult({
      name: resp.verifiedCustomer.name,
      birthDate: resp.verifiedCustomer.birthDate!,
      gender: resp.verifiedCustomer.gender!,
      operator: resp.verifiedCustomer.operator!,
      phoneNumber: resp.verifiedCustomer.phoneNumber!,
      ci: resp.verifiedCustomer.ci!,
    });
    /* eslint-enable @typescript-eslint/no-non-null-assertion */
  }

  return makeFailureResult(resp);
};

const makeSuccessResult = <T>(data: T): PortOneSuccessResult<T> => {
  return { ...data, status: 'succeeded' };
};

const makeFailureResult = (error: unknown): PortOneFailureResult => {
  if (error instanceof RestError) {
    if (error.data.type === 'PG_PROVIDER') {
      // Narrowing PgProviderError https://portone-io.github.io/server-sdk/js/types/Common.PgProviderError.html
      return { status: 'failed', code: error.data.pgCode, message: error.data.pgMessage, portoneErrorType: error.data.type };
    }
    const portoneErrorType = String(error.data.type);
    return { status: 'failed', code: portoneErrorType, message: error.message, portoneErrorType };
  }
  if (error instanceof Error) {
    return { status: 'failed', code: error.name, message: error.message, portoneErrorType: null };
  }
  return { status: 'failed', code: 'unknown', message: String(error), portoneErrorType: null };
};
